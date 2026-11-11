#!/usr/bin/env python3
"""Live-model task acceptance: real MicroVMs, host oracles, no capability skips."""

import hashlib
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
import tempfile
import time
import traceback

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))
from acp_client import AcpClient, RpcError


CASES = {
    "P4-1": "cold complex coding task with independent host oracle",
    "P4-2": "six tool-using turns retain context and evolving requirements",
    "P4-3": "125-second externally paced batch task stays in flight to completion",
    "P4-4": "external filesystem read/write, refresh, Unicode paths and fail-closed read-only handling",
    "P4-5": "named-volume session history restores in a fresh MicroVM",
    "P4-6": "concurrent sessions keep context and cancellation independent",
    "P4-7": "cancel a running process tree, prevent late writes, then recover",
    "P4-8": "invalid, conflicting and denied operations fail closed",
    "P4-9": "EOF, pool, boxes, socket and named volume clean up",
}


def record(case_id, status, detail=None):
    row = {"id": case_id, "status": status, "message": CASES[case_id], "detail": detail,
           "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
    with Path(os.environ["RESULTS_JSONL"]).open("a", encoding="utf-8") as output:
        output.write(json.dumps(row, ensure_ascii=False) + "\n")
    print(f"{case_id}: {status} — {CASES[case_id]}", flush=True)


def write_rows(path, rows):
    temporary = path.with_suffix(".tmp")
    temporary.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows), encoding="utf-8")
    temporary.replace(path)


def expected(rows, account=None):
    seen, balances, duplicates = set(), {}, 0
    for row in rows:
        if row["id"] in seen:
            duplicates += 1
            continue
        seen.add(row["id"])
        name = row["account"]
        balances[name] = balances.get(name, 0) + row["cents"] * (1 if row["kind"] == "credit" else -1)
    accounts = [{"account": k, "balanceCents": v} for k, v in sorted(balances.items())
                if account is None or k == account]
    return {"uniqueEvents": len(seen), "duplicateEvents": duplicates, "accounts": accounts,
            "netCents": sum(r["balanceCents"] for r in accounts)}


def assert_json(path, value):
    actual = json.loads(path.read_text(encoding="utf-8"))
    assert actual == value, f"{path.name}: expected {value!r}, got {actual!r}"


def invalid(client, method, params, code=-32602):
    try:
        client.request(method, params)
    except RpcError as error:
        assert error.code in ((code,) if isinstance(code, int) else code), (method, error.code, str(error))
    else:
        raise AssertionError(f"{method} accepted invalid parameters")


def main(profile=None):
    if profile is None and os.environ.get("HARNESS_PROFILE"):
        sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "e2e-acp-harnesses"))
        from profiles import Profile
        profile = Profile()
    if profile is None and not os.environ.get("DEEPSEEK_API_KEY"):
        for case_id in CASES:
            record(case_id, "BLOCKED", {"reason": "DEEPSEEK_API_KEY is not set"})
        return 1
    kbox, image = os.environ["KBOX"], os.environ["IMAGE"]
    evidence = Path(os.environ["E2E_OUT"]).resolve() / "p4-tasks"
    evidence.mkdir(parents=True, exist_ok=False)
    workspace, external, readonly = [evidence / name for name in ("工作 区", "外部 数据", "只读 数据")]
    for directory in (workspace / "out", external / "inbox", readonly):
        directory.mkdir(parents=True, exist_ok=True)
    volume = "dsh-acp-task-" + secrets.token_hex(6)
    mounts = ["-v", f"{workspace}:/workspace", "-v", f"{external}:/external",
              "-v", f"{volume}:/data"]
    clients, passed, turns = [], set(), []
    pool = pool_log = None
    socket_dir = tempfile.TemporaryDirectory(prefix="kbox-dsh-", dir="/tmp")
    socket = str(Path(socket_dir.name) / "pool.sock")
    failure = None

    def command(*args, timeout=60):
        return subprocess.run([kbox, *args], capture_output=True, text=True, check=True, timeout=timeout)

    def passed_case(case_id, detail=None):
        record(case_id, "PASS", detail)
        passed.add(case_id)

    def prompt(client, session, text, **kwargs):
        result = client.prompt(session, text, **kwargs)
        turns.append({k: v for k, v in result.items() if k != "reply"})
        print(f"  turn {len(turns)}: {result['tool_calls']} tools, {result['elapsed_s']}s", flush=True)
        return result

    def wait_ready():
        deadline = time.monotonic() + 240
        while time.monotonic() < deadline:
            assert pool.poll() is None, f"pool exited {pool.returncode}"
            status = subprocess.run([kbox, "pool", "status", "--socket", socket, "--json"],
                                    capture_output=True, text=True, timeout=10)
            if status.returncode == 0 and any(r["idle"] >= 1 for r in json.loads(status.stdout)):
                return
            time.sleep(1)
        raise TimeoutError("process-prewarmed pool never became ready")

    try:
        command("volume", "create", volume)
        volume_info = json.loads(command("volume", "inspect", volume).stdout)[0]
        assert volume_info["Name"] == volume and Path(volume_info["Mountpoint"]).is_absolute()
        # pool start accepts host paths; run alone resolves bare named-volume tokens.
        pool_mounts = [f"{volume_info['Mountpoint']}:/data" if v == f"{volume}:/data" else v for v in mounts]
        rng = secrets.SystemRandom()
        rows = [{"id": f"event-{i}", "account": ["alpha", "beta", "账户甲"][i % 3],
                 "cents": rng.randrange(0, 100_000), "kind": "debit" if i % 4 == 0 else "credit"}
                for i in range(60)]
        rows += [rows[3], rows[17], rows[3]]
        write_rows(external / "订单 数据.jsonl", rows)
        readonly_text = "immutable-" + secrets.token_hex(16) + "\n"
        (readonly / "policy.txt").write_text(readonly_text)
        ro_probe = subprocess.run([kbox, "run", "--rm", "--entrypoint", "/bin/true", "-v",
                                   f"{readonly}:/readonly:ro", image], capture_output=True, text=True, timeout=90)
        (evidence / "readonly-probe.log").write_text(ro_probe.stdout + ro_probe.stderr)
        ro_supported = ro_probe.returncode == 0
        if ro_supported:
            mounts += ["-v", f"{readonly}:/readonly:ro"]
            pool_mounts += ["-v", f"{readonly}:/readonly:ro"]
        else:
            assert "MicroVM :ro volume requires host-enforced virtio-fs write denial" in ro_probe.stderr, ro_probe.stderr
        assert (readonly / "policy.txt").read_text() == readonly_text
        binary = os.urandom(256 * 1024)
        (external / "数据.bin").write_bytes(binary)
        canary = Path(socket_dir.name) / "host-private.txt"
        canary.write_text("unmounted-" + secrets.token_hex(16))
        (workspace / "host-escape").symlink_to(canary)
        token_a, token_b = "memory-A-" + secrets.token_hex(12), "memory-B-" + secrets.token_hex(12)
        requirements = """Build a dependency-free Node ESM ledger CLI in /workspace.
Files: ledger.mjs (export the reducer), reconcile.mjs (CLI), test-ledger.mjs (node:test).
CLI: node reconcile.mjs INPUT OUTPUT [--account NAME]. Input is UTF-8 JSONL of
{id: nonempty string, account: nonempty string, cents: nonnegative safe integer, kind: credit|debit}.
Blank lines are ignored. Deduplicate identical event ids; conflicting records sharing an id are invalid.
Credits add cents, debits subtract cents. Use integer arithmetic. Reject malformed input and invalid fields
with nonzero exit BEFORE replacing OUTPUT; never destroy an existing output on failure.
Output JSON exactly: {uniqueEvents,duplicateEvents,accounts:[{account,balanceCents}],netCents}.
Accounts are sorted by Unicode code point. --account filters accounts and netCents, but event counts
always describe the whole input. Create output parent directories. Include meaningful tests of duplicate
events, integer arithmetic, invalid input, filtering and preservation of an existing output.
Only synthetic files are mounted. /external is a separate host read/write mount.
Do not read process environment, credentials, /data, or unrelated files. Do not install dependencies.
"""
        (workspace / "REQUIREMENTS.md").write_text(requirements)
        cold = profile.client(mounts, evidence, "cold") if profile else AcpClient([
            kbox, "run", "-i", "--rm", "--memory", "2g", "-u", "root",
            *mounts, "-e", "DEEPSEEK_API_KEY", image], evidence, "cold")
        clients.append(cold)
        initialized = cold.initialize()
        (evidence / "initialize.json").write_text(json.dumps(initialized, indent=2) + "\n")
        live_boxes = json.loads(command("ps", "--format", "json").stdout)
        cold_box = getattr(cold, "execution_id", None)
        if cold_box:
            assert sum(box["id"] == cold_box for box in live_boxes) == 1, live_boxes
        else:
            assert len(live_boxes) == 1, live_boxes
            cold_box = live_boxes[0]["id"]
        session = cold.new_session()
        cold.configure(session)
        cold.allow_permissions = True
        first = prompt(cold, session, f"这是一次真实编码任务。先记住仅用于会话记忆的校验口令 {token_a}，"
                       "不要把口令写入工作区或外部文件。阅读 /workspace/REQUIREMENTS.md，完成所有实现和测试，"
                       "实际运行 node --test test-ledger.mjs，再用 CLI 处理 '/external/订单 数据.jsonl'，"
                       "将结果保存为 '/workspace/out/汇总 report.json'。不要只给出计划。", timeout=600)
        assert first["tool_calls"] >= 3, first
        for name in ("ledger.mjs", "reconcile.mjs", "test-ledger.mjs"):
            assert (workspace / name).is_file(), name
        assert_json(workspace / "out/汇总 report.json", expected(rows))
        tests = command("exec", "--timeout", "60", "-w", "/workspace", cold_box, "--",
                        "node", "--test", "test-ledger.mjs", timeout=90)
        (evidence / "guest-node-tests.log").write_text(tests.stdout + tests.stderr)
        holdout = [{"id": secrets.token_hex(6), "account": "hidden", "cents": rng.randrange(100_000),
                    "kind": "credit"} for _ in range(20)]
        holdout += [holdout[7], holdout[2]]
        write_rows(external / "holdout.jsonl", holdout)
        command("exec", "--timeout", "30", "-w", "/workspace", cold_box, "--", "node", "reconcile.mjs",
                "/external/holdout.jsonl", "/workspace/out/holdout.json")
        assert_json(workspace / "out/holdout.json", expected(holdout))
        passed_case("P4-1", {"independent_guest_exec": True, **{k: v for k, v in first.items() if k != "reply"}})

        rows += [{"id": "host-added", "account": "账户甲", "cents": 199, "kind": "debit"}, rows[8]]
        write_rows(external / "订单 数据.jsonl", rows)
        second = prompt(cold, session, "宿主已更新同一外部输入文件。重新读取它，实际运行 CLI，保存为 "
                        "/workspace/out/round2.json。回复时给出最初记住的校验口令；不要从文件获取口令。")
        assert token_a in second["reply"]
        assert_json(workspace / "out/round2.json", expected(rows))
        prompt(cold, session, "接着验证 --account '账户甲' 的过滤行为，必要时修正实现，重跑测试并实际运行 CLI，"
               "将过滤结果写入 /workspace/out/filtered.json，未过滤结果写入 /workspace/out/unfiltered.json。")
        assert_json(workspace / "out/filtered.json", expected(rows, "账户甲"))
        assert_json(workspace / "out/unfiltered.json", expected(rows))
        protected = "preserve-existing-output-" + secrets.token_hex(12)
        (workspace / "out/protected.json").write_text(protected)
        (external / "malformed.jsonl").write_text(json.dumps(rows[0]) + '\n{"truncated":\n')
        prompt(cold, session, "做一次失败恢复验证。用 CLI 处理 /external/malformed.jsonl，输出目标为现有的 "
               "/workspace/out/protected.json。命令必须非零退出且原文件字节完全不变。必要时修复代码，"
               "运行测试。把实际子进程退出码保存为 /workspace/out/invalid-check.json，格式 {\"exitCode\":数字}。")
        assert (workspace / "out/protected.json").read_text() == protected
        assert json.loads((workspace / "out/invalid-check.json").read_text())["exitCode"] != 0
        rejected = subprocess.run([kbox, "exec", "--timeout", "30", "-w", "/workspace", cold_box, "--",
                                   "node", "reconcile.mjs", "/external/malformed.jsonl",
                                   "/workspace/out/protected.json"], capture_output=True, text=True, timeout=45)
        assert rejected.returncode != 0, "independent malformed-input check succeeded"
        assert (workspace / "out/protected.json").read_text() == protected
        (evidence / "guest-invalid-input.log").write_text(rejected.stdout + rejected.stderr)

        stream_start, produced, progress = [None], [], [0]
        watch_marker = workspace / "out/watch-started.json"

        def feed():
            if stream_start[0] is None:
                if not watch_marker.exists():
                    return
                stream_start[0] = time.monotonic()
                print("  stream: worker started; beginning paced host input", flush=True)
            elapsed = time.monotonic() - stream_start[0]
            while len(produced) < 12 and elapsed >= len(produced) * 10:
                index = len(produced)
                batch = [{"id": f"batch-{index}-{i}", "account": "alpha" if i % 2 else "账户甲",
                          "cents": rng.randrange(1, 9999), "kind": "debit" if i % 3 else "credit"}
                         for i in range(8)]
                batch.append(batch[2])
                name = f"batch-{index:02}.jsonl"
                write_rows(external / "inbox" / name, batch)
                produced.append({"batch": name, "summary": expected(batch)})
            if elapsed >= 125:
                (external / "inbox/DONE").touch(exist_ok=True)
            if elapsed >= progress[0] + 20:
                progress[0] = elapsed
                print(f"  stream: {elapsed:.0f}s, {len(produced)}/12 host batches", flush=True)

        long_turn = prompt(cold, session,
            "现在完成长程流式任务：实现 /workspace/watch.mjs，复用 ledger.mjs。宿主会每10秒向 "
            "/external/inbox 原子写入 batch-NN.jsonl，共12个文件，125秒后写入 DONE。"
            "watch.mjs 启动后先将 {at:Date.now()} 写入 /workspace/out/watch-started.json；"
            "宿主看到该启动标记才开始上述125秒计时和投递。"
            "扫描新批次，每个仅处理一次，按 batch 文件名排序，最终写入 /workspace/out/stream.json，"
            "格式为 [{batch:文件名,summary:与CLI相同的汇总}]。发现 DONE 后处理完所有批次再退出。"
            "必须实际启动 node watch.mjs 并等待它完成，再读回输出并回复。"
            "bash 命令用足够的 timeout_ms（例如240000）；如果转为后台任务必须持续轮询直至退出，不能提前结束本轮。",
            timeout=600, tick=feed)
        assert long_turn["elapsed_s"] >= 125, long_turn
        assert stream_start[0] is not None and time.monotonic() - stream_start[0] >= 125
        assert len(produced) == 12
        assert_json(workspace / "out/stream.json", produced)
        passed_case("P4-3", {"batches": len(produced), **{k: v for k, v in long_turn.items() if k != "reply"}})

        refresh = "host-refresh-" + secrets.token_hex(12) + "\n中文外部数据\n"
        (external / "实时 输入.txt").write_text(refresh)
        readonly_task = (
            "实际尝试用 Node fs.writeFileSync 修改 /readonly/policy.txt，捕获真实错误码（不得模拟），"
            "将 {code:错误码} 写入 /workspace/out/readonly-check.json。只读挂载必须拒绝写入。"
        ) if ro_supported else "本后端已经在入口明确拒绝了只读挂载，/readonly 未挂载；无需访问它。"
        external_turn = prompt(cold, session,
            "验证另一个宿主文件系统：读取 /external/实时 输入.txt，以原始UTF-8字节复制到 "
            "'/external/外部 输出/临时.txt'，再重命名为 '/external/外部 输出/最终.txt'，"
            "并在 /workspace/out/refresh.txt 保存同样内容。" + readonly_task + "以原始二进制复制 /external/数据.bin "
            "到 /workspace/out/复制.bin。实际尝试读取 /workspace/host-escape（指向未挂载宿主路径的符号链接），"
            "把真实失败码保存为 /workspace/out/escape-check.json，格式 {code:错误码}。"
            "需要外部写权限时可申请一次性批准。")
        assert (external / "外部 输出/最终.txt").read_text() == refresh
        assert not (external / "外部 输出/临时.txt").exists()
        assert (workspace / "out/refresh.txt").read_text() == refresh
        assert (readonly / "policy.txt").read_text() == readonly_text
        assert (workspace / "out/复制.bin").read_bytes() == binary
        assert json.loads((workspace / "out/escape-check.json").read_text())["code"] in ("ENOENT", "EACCES", "EPERM")
        denial = "unsupported mount rejected before boot"
        if ro_supported:
            denial = json.loads((workspace / "out/readonly-check.json").read_text())["code"]
            assert denial in ("EROFS", "EACCES", "EPERM"), denial
        assert external_turn["tool_calls"] > 0
        passed_case("P4-4", {"read_only_supported": ro_supported, "read_only_error": denial, "permissions": len(cold.permissions),
                                "utf8_sha256": hashlib.sha256(refresh.encode()).hexdigest()})
        assert len(turns) == 6 and all(t["tool_calls"] > 0 for t in turns)
        assert sum(t["tool_calls"] for t in turns) >= 12
        passed_case("P4-2", {"turns": turns, "total_tool_calls": sum(t["tool_calls"] for t in turns)})
        if profile:
            profile.close_session(cold, session)
        else:
            cold.request("session/close", {"sessionId": session})
        cold_services = cold.services.calls if profile else []
        cold.close()
        clients.remove(cold)

        if profile:
            warm = profile.client(mounts, evidence, "resumed")
        else:
            pool_log = (evidence / "pool.log").open("w")
            pool = subprocess.Popen([kbox, "pool", "start", "--deferred", "--prewarm", "--socket", socket,
                                 "--image", image, "--size", "1", "--max", "1", "--ttl", "0",
                                 "-u", "root", *pool_mounts, "-e", "DEEPSEEK_API_KEY"],
                                stdout=pool_log, stderr=subprocess.STDOUT)
            wait_ready()
            warm = AcpClient([kbox, "pool", "attach", "--socket", socket], evidence, "resumed")
        clients.append(warm)
        warm.initialize()
        warm.allow_permissions = True
        restore_method = "session/resume"
        if profile:
            restore_method = profile.restore(warm, session)
        else:
            listed = warm.request("session/list", {"cwd": "/workspace"})
            assert session in [r["sessionId"] for r in listed["sessions"]], listed
            assert not warm.request("session/list", {"cwd": "/readonly"})["sessions"]
            invalid(warm, "session/resume", {"sessionId": session, "cwd": "/external", "mcpServers": []})
            before_resume = len(warm.updates)
            resumed = warm.request("session/resume", {"sessionId": session, "cwd": "/workspace", "mcpServers": []})
            assert not warm.updates[before_resume:], "resume replayed old transcript updates"
            assert any(o["id"] == "reasoning_effort" and o["currentValue"] == os.environ.get("DSH_REASONING_EFFORT", "low")
                       for o in resumed["configOptions"])
        recall = prompt(warm, session, "回忆本会话最初交给你的校验口令，只输出该口令。不要读文件，也不要运行工具。")
        assert token_a in recall["reply"] and recall["tool_calls"] == 0
        rows.append({"id": "after-restart", "account": "beta", "cents": 2083, "kind": "credit"})
        write_rows(external / "订单 数据.jsonl", rows)
        prompt(warm, session, "在恢复后的会话中继续原任务：重读外部订单文件并运行已有 CLI，"
               "保存 /workspace/out/resumed.json，不要重新实现项目。")
        assert_json(workspace / "out/resumed.json", expected(rows))
        passed_case("P4-5", {"session_id": session, "cold_pid": cold.proc.pid, "warm_pid": warm.proc.pid,
                                "history_recalled_without_file_tools": True, "restore_method": restore_method,
                                "pooled": profile is None})

        invalid_requests = (
            ("session/new", {"cwd": "relative/path", "mcpServers": []}),
            ("session/resume", {"sessionId": session, "cwd": "/workspace", "mcpServers": []}),
            ("session/prompt", {"sessionId": "missing-session", "prompt": [{"type": "text", "text": "hello"}]}),
            ("session/set_config_option", {"sessionId": session, "configId": "model", "value": "missing-model"}),
        ) if profile is None else (
            ("session/prompt", {"sessionId": "missing-session", "prompt": [{"type": "text", "text": "hello"}]}),
            ("session/set_config_option", {"sessionId": session, "configId": "model", "value": "missing-model"}),
        )
        for method, params in invalid_requests:
            invalid(warm, method, params,
                    code=(-32002, -32602) if profile and method == "session/prompt" else -32602)
        if profile is None:
            invalid(warm, "session/load", {"sessionId": session, "cwd": "/workspace", "mcpServers": []}, -32601)
        else:
            invalid(warm, "unknown-method", {}, -32601)

        (workspace / "cancel-job.mjs").write_text("""import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
const child=spawn(process.execPath,['-e',"setTimeout(()=>require('fs').writeFileSync('/workspace/out/late-write.txt','late'),45000)"],{stdio:'ignore'});
writeFileSync('/workspace/out/cancel-started.json',JSON.stringify({pid:process.pid,childPid:child.pid,at:Date.now()}));
setTimeout(()=>writeFileSync('/workspace/out/late-parent.txt','late'),180000);
""")
        pending_start = len(warm.updates)
        pending = warm.send("session/prompt", {"sessionId": session, "prompt": [{"type": "text", "text":
            "实际用 bash 前台运行 node /workspace/cancel-job.mjs，timeout_ms=240000，等待结束。"
            "这是可取消的测试进程，不要修改脚本，不要使用 detached/nohup。"}]})
        deadline = time.monotonic() + 90
        marker = workspace / "out/cancel-started.json"
        while not marker.exists():
            assert time.monotonic() < deadline, "agent never started cancellable command"
            assert pending not in warm.responses, "prompt returned before starting long command"
            warm.pump()
        live_marker = json.loads(marker.read_text())
        if profile is None:
            invalid(warm, "session/prompt", {"sessionId": session, "prompt": [{"type": "text", "text": "overlap"}]})
        other = warm.new_session()
        warm.configure(other)
        other_reply = prompt(warm, other, f"只在本会话中记住校验口令 {token_b}。只回复该口令，不用工具。")
        assert token_b in other_reply["reply"] and token_a not in other_reply["reply"]
        assert pending not in warm.responses, "first session did not remain active during second session"
        cancelled_at = time.monotonic()
        warm.send("session/cancel", {"sessionId": session}, notification=True)
        result = warm.wait(pending, timeout=15)
        cancel_latency = time.monotonic() - cancelled_at
        assert result["stopReason"] == "cancelled", result
        cancelled_updates = [u["update"] for u in warm.updates[pending_start:] if u["sessionId"] == session]
        cancelled_calls = {u["toolCallId"] for u in cancelled_updates if u["sessionUpdate"] == "tool_call"}
        settled_calls = {u["toolCallId"] for u in cancelled_updates if u["sessionUpdate"] == "tool_call_update"
                         and u.get("status") in ("completed", "failed")}
        assert cancelled_calls and cancelled_calls <= settled_calls, "cancel returned before tool-result drain"
        other_recall = prompt(warm, other, "我在这个会话里刚才给你的校验口令是什么？只输出它，不用工具。")
        assert token_b in other_recall["reply"] and token_a not in other_recall["reply"]
        passed_case("P4-6", {"sessions": [session, other], "overlapping_prompt_rejected": profile is None,
                                "other_session_survived_cancel": True})
        late_deadline = live_marker["at"] / 1000 + 50
        while time.time() < late_deadline:
            assert not (workspace / "out/late-write.txt").exists(), "cancelled child wrote after cancellation"
            assert not (workspace / "out/late-parent.txt").exists(), "cancelled parent wrote after cancellation"
            warm.pump()
        prompt(warm, session, "取消已经完成。请实际运行 Node 将 {\"recovered\":true} 写入 "
               "/workspace/out/recovered.json，然后回复最初的本会话校验口令。")
        assert_json(workspace / "out/recovered.json", {"recovered": True})
        assert not (workspace / "out/late-write.txt").exists()
        passed_case("P4-7", {"cancel_latency_s": round(cancel_latency, 3), "late_write_observation_s": 50,
                                "process_tree": live_marker})

        warm.allow_permissions = False
        if profile:
            profile.prepare_denial(warm, session)
        permission_start = len(warm.permissions)
        denial_prompt = (
               "进行权限拒绝测试：只使用 write 工具尝试创建 /external/permission-denied.txt，content 为 denied，"
               "sandbox_permissions 必须设为 danger-full-access，justification 为一次性外部写权限测试。"
               "如果客户端拒绝就停止并报告，不要使用其它工具绕过拒绝。") if profile is None or profile.name == "deepseek" else (
               "进行客户端权限拒绝测试。用你实际具备的文件写入工具尝试创建 /external/permission-denied.txt，"
               "内容为 denied。此路径在工作区外，需要先向客户端请求一次性写入批准。客户端会拒绝；"
               "被拒绝后立即停止，不要重试，也不要用其它工具或命令绕过拒绝。")
        prompt(warm, session, denial_prompt)
        denied = warm.permissions[permission_start:]
        assert denied and all(p["outcome"].get("outcome") == "cancelled" or
            any(o["optionId"] == p["outcome"].get("optionId") and o["kind"] == "reject_once"
                for o in p["request"]["options"]) for p in denied), denied
        assert not (external / "permission-denied.txt").exists(), "rejected permission had filesystem effects"
        passed_case("P4-8", {"typed_invalid_requests": 7 if profile is None else 3, "rejected_permissions": len(denied)})
        if profile:
            profile.close_session(warm, other)
            profile.close_session(warm, session)
            (evidence / "client-services.json").write_text(json.dumps(cold_services + warm.services.calls, indent=2) + "\n")
        else:
            warm.request("session/close", {"sessionId": other})
            warm.request("session/close", {"sessionId": session})
        warm.close()
        clients.remove(warm)
    except Exception as error:
        failure = error
        (evidence / "failure.txt").write_text(traceback.format_exc())
        print(traceback.format_exc(), file=sys.stderr, flush=True)
    finally:
        cleanup_errors = []
        for client in clients:
            try:
                client.close(strict=False)
            except Exception as error:
                cleanup_errors.append(str(error))
        if pool is not None:
            try:
                command("pool", "stop", "--socket", socket, timeout=90)
                assert pool.wait(timeout=30) == 0
            except Exception as error:
                cleanup_errors.append(str(error))
                pool.terminate()
                try:
                    pool.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    pool.kill()
                    pool.wait(timeout=10)
        if pool_log is not None:
            pool_log.close()
        try:
            boxes = json.loads(command("ps", "--all", "--format", "json").stdout)
            (evidence / "boxes-after.json").write_text(json.dumps(boxes, indent=2) + "\n")
            assert not any(b["raw_status"] in ("running", "paused") for b in boxes), "live boxes remain"
            assert not Path(socket).exists(), "pool socket remains"
            command("volume", "rm", volume)
            volumes = command("volume", "ls", "--quiet").stdout.splitlines()
            assert volume not in volumes, "named volume remains"
            (evidence / "volumes-after.json").write_text(json.dumps(volumes, indent=2) + "\n")
        except Exception as error:
            cleanup_errors.append(str(error))
        socket_dir.cleanup()
        record("P4-9", "FAIL" if cleanup_errors else "PASS", {"errors": cleanup_errors})
        for case_id in CASES:
            if case_id != "P4-9" and case_id not in passed:
                record(case_id, "FAIL", {"reason": str(failure or "required assertion not reached")})
        (evidence / "turns.json").write_text(json.dumps(turns, indent=2) + "\n")
        # Diagnostics can contain provider errors; redact inherited secrets before retaining evidence.
        for path in evidence.glob("*.log"):
            text = path.read_text(errors="replace")
            for secret in ((profile.redactions if profile else []) +
                           [os.environ.get("DEEPSEEK_API_KEY"), os.environ.get("OPENAI_API_KEY")]):
                if secret:
                    text = text.replace(secret, "<redacted>")
            path.write_text(text)
    return 1 if failure or cleanup_errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
