#!/usr/bin/env python3
"""Paired long-horizon ACP benchmark: kbox MicroVM vs bare host dsh-acp.

Design principles
-----------------
* The *same* 10 task definitions (identical prompt bytes, identical seed bytes,
  identical node commands) run on both backends.
* Every task is a 3-turn, dependent pipeline in one ACP session: step 1 generates
  data, step 2 transforms it, step 3 waits in flight and proves the chain.
* The host independently runs every command in a scratch directory to produce the
  reference bytes and a pure-host compute-time sample. Backend output must match
  byte-for-byte; "PASS" is an oracle verdict, not a model opinion.
* LLM variance is deliberately minimised (the prompt gives an exact command and
  asks for a one-word reply). What remains is the infrastructure delta.
* Resource sampling uses macOS libproc directly, so it works in sandboxed shells
  where `ps` is denied. No external Python packages are required.

The DeepSeek credential is read from config.acl at runtime, passed only into
child process environments, and redacted from every ACP trace.
"""

import ctypes
import ctypes.util
import hashlib
import json
import os
from pathlib import Path
import random
import re
import statistics
import subprocess
import sys
import threading
import time

ROOT = Path("/Users/roylin/code/kense/kbox")
EVIDENCE = ROOT / "docs/evidence/box/20261111-longrun-kbox-vs-bare"
TRACES = EVIDENCE / "traces"
RUNTIME = Path("/tmp/kbox-longrun-bench")
# Keep all bind-mount / socket-adjacent paths short: macOS sun_path is 104 bytes.
WORKSPACES = RUNTIME / "ws"
K_HOME_WARM = Path("/tmp/kbox-e2e-ad-release/k-home")
KBOX = ROOT / "target/release/kbox"
IMAGE = "ghcr.io/kense-labs/dsh-acp:latest"
BARE_BIN = Path("/Users/roylin/code/kense/dsh-acp/apps/cli/lib/bin.js")
NODE = "/Users/roylin/.workbuddy/binaries/node/versions/22.22.2-6/bin/node"
CONFIG_ACL = Path("/Users/roylin/code/config.acl")

sys.path.insert(0, str(ROOT / "scripts/e2e-dsh-acp/lib"))
from acp_client import AcpClient  # noqa: E402


# --------------------------------------------------------------------------
# macOS libproc resource sampling (works without `ps`)
# --------------------------------------------------------------------------
class RUsageInfoV3(ctypes.Structure):
    _fields_ = [
        ("ri_uuid", ctypes.c_ubyte * 16),
        ("ri_user_time", ctypes.c_uint64), ("ri_system_time", ctypes.c_uint64),
        ("ri_pkg_idle_wkups", ctypes.c_uint64), ("ri_interrupt_wkups", ctypes.c_uint64),
        ("ri_pageins", ctypes.c_uint64), ("ri_wired_size", ctypes.c_uint64),
        ("ri_resident_size", ctypes.c_uint64), ("ri_phys_footprint", ctypes.c_uint64),
        ("ri_proc_start_abstime", ctypes.c_uint64), ("ri_proc_exit_abstime", ctypes.c_uint64),
        ("ri_child_user_time", ctypes.c_uint64), ("ri_child_system_time", ctypes.c_uint64),
        ("ri_child_pkg_idle_wkups", ctypes.c_uint64), ("ri_child_interrupt_wkups", ctypes.c_uint64),
        ("ri_child_pageins", ctypes.c_uint64), ("ri_child_elapsed_abstime", ctypes.c_uint64),
        ("ri_diskio_bytesread", ctypes.c_uint64), ("ri_diskio_byteswrite", ctypes.c_uint64),
        ("ri_cpu_time_qos_default", ctypes.c_uint64), ("ri_cpu_time_qos_maintenance", ctypes.c_uint64),
        ("ri_cpu_time_qos_background", ctypes.c_uint64), ("ri_cpu_time_qos_utility", ctypes.c_uint64),
        ("ri_cpu_time_qos_legacy", ctypes.c_uint64), ("ri_cpu_time_qos_user_initiated", ctypes.c_uint64),
        ("ri_cpu_time_qos_user_interactive", ctypes.c_uint64), ("ri_billed_system_time", ctypes.c_uint64),
        ("ri_serviced_system_time", ctypes.c_uint64), ("ri_logical_writes", ctypes.c_uint64),
        ("ri_lifetime_max_phys_footprint", ctypes.c_uint64), ("ri_instructions", ctypes.c_uint64),
        ("ri_cycles", ctypes.c_uint64), ("ri_billed_energy", ctypes.c_uint64),
        ("ri_serviced_energy", ctypes.c_uint64), ("ri_interval_max_phys_footprint", ctypes.c_uint64),
        ("ri_runnable_time", ctypes.c_uint64), ("ri_flags", ctypes.c_uint64),
    ]


LIBPROC = ctypes.CDLL(ctypes.util.find_library("proc"))
LIBSYSTEM = ctypes.CDLL(ctypes.util.find_library("System"))


class MachTimebase(ctypes.Structure):
    _fields_ = [("numer", ctypes.c_uint32), ("denom", ctypes.c_uint32)]


_TIMEBASE = MachTimebase()
LIBSYSTEM.mach_timebase_info(ctypes.byref(_TIMEBASE))
# proc_pid_rusage reports CPU times in mach absolute ticks, not nanoseconds.
_TICKS_TO_NS = _TIMEBASE.numer / _TIMEBASE.denom

LIBPROC.proc_pid_rusage.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.POINTER(RUsageInfoV3)]
LIBPROC.proc_pid_rusage.restype = ctypes.c_int
LIBPROC.proc_listallpids.argtypes = [ctypes.c_void_p, ctypes.c_int]
LIBPROC.proc_listallpids.restype = ctypes.c_int
LIBPROC.proc_listchildpids.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_int]
LIBPROC.proc_listchildpids.restype = ctypes.c_int
LIBPROC.proc_pidpath.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_uint32]
LIBPROC.proc_pidpath.restype = ctypes.c_int


def rusage(pid):
    info = RUsageInfoV3()
    if LIBPROC.proc_pid_rusage(pid, 3, ctypes.byref(info)) != 0:
        return None
    return (info.ri_resident_size / 1048576.0,
            (info.ri_user_time + info.ri_system_time) * _TICKS_TO_NS / 1e9)


def all_pids():
    n = LIBPROC.proc_listallpids(None, 0)
    if n <= 0:
        return []
    arr = (ctypes.c_int * n)()
    m = LIBPROC.proc_listallpids(arr, ctypes.sizeof(arr))
    return list(arr)[:m]


def pid_path(pid):
    buf = ctypes.create_string_buffer(4096)
    if LIBPROC.proc_pidpath(pid, buf, 4096) > 0:
        return buf.value.decode("utf-8", "replace")
    return ""


def descendants(root, pool):
    out, frontier = set(), [root]
    while frontier:
        pid = frontier.pop()
        if pid in out:
            continue
        out.add(pid)
        kids = (ctypes.c_int * 256)()
        n = LIBPROC.proc_listchildpids(pid, kids, ctypes.sizeof(kids))
        frontier.extend(c for c in list(kids)[:max(n, 0)] if c in pool)
    return out


class ResourceSampler:
    """Accumulate CPU-seconds and peak RSS for one backend's process tree."""

    def __init__(self, backend, root_pid, interval=0.2):
        self.backend = backend
        self.root_pid = root_pid
        self.interval = interval
        self.lock = threading.Lock()
        self.baseline = {}
        self.cpu_seconds = 0.0
        self.peak_rss_mb = 0.0
        self.rss_samples = []
        self.max_processes = 0
        self.samples = 0
        self._stop = threading.Event()
        self.thread = threading.Thread(target=self._run, daemon=True)
        self.path_cache = {}

    def _pids(self):
        pool = set(all_pids())
        if self.backend == "kbox":
            hits = set()
            for p in pool:
                if p not in self.path_cache:
                    self.path_cache[p] = pid_path(p).lower()
                if "kbox" in self.path_cache[p] or "krun" in self.path_cache[p]:
                    hits.add(p)
            hits.add(self.root_pid)
            return hits
        return descendants(self.root_pid, pool)

    def _run(self):
        while not self._stop.is_set():
            try:
                pids = self._pids()
                total_rss = 0.0
                for pid in pids:
                    reading = rusage(pid)
                    if reading is None:
                        continue
                    rss, cpu = reading
                    total_rss += rss
                    with self.lock:
                        if pid in self.baseline:
                            delta = cpu - self.baseline[pid]
                            if 0 <= delta < 30:
                                self.cpu_seconds += delta
                        self.baseline[pid] = cpu
                with self.lock:
                    self.peak_rss_mb = max(self.peak_rss_mb, total_rss)
                    self.rss_samples.append(total_rss)
                    self.max_processes = max(self.max_processes, len(pids))
                    self.samples += 1
            except Exception:
                pass
            self._stop.wait(self.interval)

    def start(self):
        self.thread.start()

    def stop(self):
        self._stop.set()
        self.thread.join(timeout=5)
        with self.lock:
            return {"cpu_seconds": round(self.cpu_seconds, 3),
                    "peak_rss_mb": round(self.peak_rss_mb, 1),
                    "mean_rss_mb": round(statistics.mean(self.rss_samples), 1) if self.rss_samples else 0.0,
                    "max_processes": self.max_processes,
                    "samples": self.samples}


# --------------------------------------------------------------------------
# Deterministic workload: one node -e script, three dependent steps
# --------------------------------------------------------------------------
JS = r'''const fs=require("fs"),z=require("zlib"),c=require("crypto");
const mode=process.argv[1],step=+process.argv[2],burn=+process.argv[3];
const H=s=>c.createHash("sha256").update(s).digest("hex");
const rd=f=>fs.readFileSync(f,"utf8");
const wr=(f,v)=>fs.writeFileSync(f,typeof v==="string"?v:JSON.stringify(v));
let x=1;for(let i=0;i<burn;i++)x=(Math.imul(x,1664525)+1013904223)>>>0;
const seed=JSON.parse(rd("input/seed.json"));
if(step===1){const rows=[];for(let i=0;i<seed.n;i++)rows.push({k:(seed.k+i*seed.m)%seed.mod,v:(i*7919+seed.v)%10007,s:seed.salt+i});wr("out/step1.json",{mode,n:rows.length,rows,digest:H(JSON.stringify(rows)),x});}
else if(step===2){const a=JSON.parse(rd("out/step1.json"));let o;
if(mode==="sum"){o={mode,sum:a.rows.reduce((p,r)=>p+r.v,0),count:a.n};}
else if(mode==="sort"){const b=a.rows.slice().sort((p,q)=>p.k-q.k||p.v-q.v);o={mode,first:b.slice(0,5),digest:H(JSON.stringify(b))};}
else if(mode==="group"){const g={};for(const r of a.rows)g[r.k]=(g[r.k]||0)+r.v;const e=Object.entries(g).sort();o={mode,groups:e.length,digest:H(JSON.stringify(e))};}
else if(mode==="gzip"){const s=JSON.stringify(a.rows),g=z.gzipSync(s),b=z.gunzipSync(g).toString();o={mode,roundtrip:b===s,plain:s.length,digest:H(b)};}
else if(mode==="primes"){const lim=seed.limit,s=new Uint8Array(lim+1);for(let i=2;i*i<=lim;i++)if(!s[i])for(let j=i*i;j<=lim;j+=i)s[j]=1;let cnt=0,tot=0;for(const r of a.rows){const v=r.k%lim;if(v>=2&&!s[v]){cnt++;tot+=v}}o={mode,count:cnt,total:tot,lim};}
else if(mode==="matrix"){const n=seed.msize,A=new Float64Array(n*n),B=new Float64Array(n*n);for(let i=0;i<n*n;i++){A[i]=(i*13+seed.k)%97;B[i]=(i*7+seed.v)%89}const C=new Float64Array(n*n);for(let i=0;i<n;i++)for(let k=0;k<n;k++){const t=A[i*n+k];for(let j=0;j<n;j++)C[i*n+j]+=t*B[k*n+j]}o={mode,n,trace:Array.from({length:n},(_,i)=>C[i*n+i]),digest:H(JSON.stringify(Array.from(C)))};}
else if(mode==="chain"){let h=seed.salt;for(const r of a.rows)h=H(h+":"+r.k+":"+r.v);o={mode,head:h,rounds:a.n};}
else if(mode==="fanout"){fs.mkdirSync("out/parts",{recursive:true});let acc="";for(let i=0;i<seed.fan;i++){const p="out/parts/p"+String(i).padStart(4,"0")+".txt",t=H(seed.salt+":"+i);fs.writeFileSync(p,t);acc+=t}o={mode,files:seed.fan,digest:H(acc)};}
else if(mode==="tree"){let cnt=0;for(let i=0;i<seed.depth;i++){const d="out/tree/l"+i;fs.mkdirSync(d,{recursive:true});for(let j=0;j<seed.width;j++){fs.writeFileSync(d+"/f"+j+".txt",H(seed.salt+":"+i+":"+j));cnt++}}o={mode,files:cnt,digest:H(String(cnt)+seed.salt)};}
else {const seen=new Set(),b=[];for(const r of a.rows){if(!seen.has(r.k)){seen.add(r.k);b.push(r)}}o={mode,unique:b.length,digest:H(JSON.stringify(b))};}
o.x=x;wr("out/step2.json",o);}
else{const w=seed.wait_ms;Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,w);const d=H(rd("out/step1.json")+rd("out/step2.json")+seed.salt+String(x));wr("out/step3.json",{mode,digest:d,x,waited_ms:w});}
console.log("OK "+mode+" "+step);'''

MODES = [
    ("T01", "sum", "长程聚合：2 万行事件求和"),
    ("T02", "sort", "长程排序：稳定排序与全量摘要"),
    ("T03", "group", "长程分组：账户维度归约"),
    ("T04", "gzip", "长程压缩：gzip 往返与字节校验"),
    ("T05", "primes", "长程筛法：素数判定与统计"),
    ("T06", "matrix", "长程矩阵：48×48 稠密乘法"),
    ("T07", "chain", "长程哈希链：逐行链式摘要"),
    ("T08", "fanout", "长程文件扇出：200 个小文件"),
    ("T09", "tree", "长程目录树：10×12 嵌套文件"),
    ("T10", "dedup", "长程去重：按主键消歧"),
]


def make_seed(index):
    rng = random.Random(91000 + index)
    return {"n": 20000, "k": rng.randrange(1000, 9000), "m": rng.randrange(3, 97),
            "v": rng.randrange(100, 900), "mod": rng.randrange(20000, 60000),
            "salt": hashlib.sha256(f"longrun-seed-{index}".encode()).hexdigest()[:16],
            "limit": 200003, "msize": 48, "fan": 200, "depth": 10, "width": 12,
            "wait_ms": 1200 + (index % 4) * 300,
            "burn": 600_000_000 + index * 150_000_000}


def command(mode, step, burn):
    return "node -e '" + JS + "' " + mode + " " + str(step) + " " + str(burn)


def prompt_for(index, step, cmd):
    return ("这是一个固定基准任务的第 " + str(step) + "/3 步（任务编号 " + str(index + 1) + "）。\n"
            "必须使用 shell 工具原样执行下面这一整条命令，不要改写、不要拆分、不要只给计划：\n"
            + cmd + "\n"
            "命令执行成功后只回复 STEP" + str(step) + "_DONE，不要输出其它内容。")


def oracle_paths(step):
    return [f"out/step{k}.json" for k in range(1, step + 1)]


# --------------------------------------------------------------------------
# Credential + backend construction
# --------------------------------------------------------------------------
def credential():
    text = CONFIG_ACL.read_text(encoding="utf-8")
    block = re.search(r'providers\s+"deepseek"\s*\{(.*?)\n\}', text, re.S).group(1)
    return re.search(r'api_key\s*=\s*"([^"]+)"', block).group(1)


def base_env(key, *, k_home=K_HOME_WARM, dsh_home=None):
    env = dict(os.environ)
    env.update({"DEEPSEEK_API_KEY": key, "DSH_MODEL": "deepseek-flash",
                "DSH_REASONING_EFFORT": "low", "DSH_TELEMETRY_DISABLED": "1",
                "NODE_ENV": "production", "K_HOME": str(k_home)})
    if dsh_home is not None:
        env["DSH_HOME"] = str(dsh_home)
    return env


def make_command(backend, workspace, key, k_home=K_HOME_WARM):
    if backend == "bare":
        return [NODE, str(BARE_BIN), "--profile", "acp"], str(workspace)
    return ([str(KBOX), "run", "-i", "--rm", "--memory", "2g", "-u", "root",
             "-v", f"{workspace}:/workspace", "-e", "DEEPSEEK_API_KEY", IMAGE],
            "/workspace")


# --------------------------------------------------------------------------
# ACP event instrumentation
# --------------------------------------------------------------------------
def attach_timing(client):
    events = []
    original = client._log

    def logged(direction, message):
        events.append((time.monotonic(), direction, message))
        return original(direction, message)

    client._log = logged
    return events


def rpc_rtt(events, method):
    sent = next((t for t, d, m in events if d == "client" and m.get("method") == method), None)
    rid = next((m.get("id") for t, d, m in events if d == "client" and m.get("method") == method), None)
    if sent is None or rid is None:
        return None
    recv = next((t for t, d, m in events if d == "agent" and m.get("id") == rid
                 and ("result" in m or "error" in m) and t >= sent), None)
    return round((recv - sent) * 1000, 1) if recv else None


def turn_timing(events, since_index):
    window = events[since_index:]
    sent = next((t for t, d, m in window if d == "client" and m.get("method") == "session/prompt"), None)
    rid = next((m.get("id") for t, d, m in window if d == "client" and m.get("method") == "session/prompt"), None)
    if sent is None:
        return {}
    agents = [(t, m) for t, d, m in window if d == "agent" and t >= sent]
    updates = [(t, m) for t, m in agents if m.get("method") == "session/update"]
    done = next((t for t, m in agents if m.get("id") == rid and ("result" in m or "error" in m)), None)
    calls, settled = {}, {}
    tool_calls = tool_updates = 0
    for t, m in updates:
        u = m.get("params", {}).get("update", {})
        kind = u.get("sessionUpdate")
        if kind == "tool_call":
            tool_calls += 1
            calls[u.get("toolCallId")] = t
        elif kind == "tool_call_update":
            tool_updates += 1
            if u.get("status") in ("completed", "failed"):
                settled[u.get("toolCallId")] = t
    spans = [round(settled[k] - calls[k], 3) for k in calls if k in settled and settled[k] >= calls[k]]
    return {"ttft_ms": round((updates[0][0] - sent) * 1000, 1) if updates else None,
            "prompt_rtt_ms": round((done - sent) * 1000, 1) if done else None,
            "tool_calls": tool_calls, "tool_updates": tool_updates,
            "tool_span_s": max(spans) if spans else None,
            "updates": len(updates)}


# --------------------------------------------------------------------------
# Reference run (pure host, no ACP, no VM, no LLM)
# --------------------------------------------------------------------------
def run_reference(task, ref_dir):
    """Produce authoritative output bytes + pure host compute time."""
    (ref_dir / "input").mkdir(parents=True, exist_ok=True)
    (ref_dir / "out").mkdir(parents=True, exist_ok=True)
    (ref_dir / "input/seed.json").write_text(json.dumps(task["seed"], sort_keys=True), encoding="utf-8")
    result = {}
    for step in (1, 2, 3):
        cmd = command(task["mode"], step, task["seed"]["burn"])
        began = time.monotonic()
        ref_env = dict(os.environ)
        ref_env["PATH"] = str(Path(NODE).parent) + os.pathsep + ref_env.get("PATH", "")
        proc = subprocess.run(["/bin/sh", "-c", cmd], cwd=ref_dir, capture_output=True,
                              text=True, timeout=180, env=ref_env)
        result[f"step{step}_s"] = round(time.monotonic() - began, 3)
        result[f"step{step}_rc"] = proc.returncode
        if proc.returncode != 0:
            result[f"step{step}_stderr"] = proc.stderr[-500:]
        result[f"step{step}_bytes"] = {
            name: (ref_dir / name).read_bytes() for name in oracle_paths(step)
            if (ref_dir / name).exists()}
    return result


# --------------------------------------------------------------------------
# One backend run of one task
# --------------------------------------------------------------------------
def run_task(task, backend, key, label_prefix):
    tid, index, mode = task["id"], task["index"], task["mode"]
    workspace = WORKSPACES / f"{backend}-{tid}"
    if workspace.exists():
        import shutil
        shutil.rmtree(workspace)
    (workspace / "input").mkdir(parents=True)
    (workspace / "out").mkdir(parents=True)
    (workspace / "input/seed.json").write_text(json.dumps(task["seed"], sort_keys=True), encoding="utf-8")
    trace_dir = TRACES
    trace_dir.mkdir(parents=True, exist_ok=True)
    label = f"{label_prefix}-{backend}-{tid}"

    dsh_home = RUNTIME / "bare-home" if backend == "bare" else None
    if dsh_home is not None:
        dsh_home.mkdir(parents=True, exist_ok=True)
    env = base_env(key, dsh_home=dsh_home)
    cmd, session_cwd = make_command(backend, workspace, key)
    record = {"task": tid, "index": index, "mode": mode, "backend": backend,
              "label": label, "workspace": str(workspace), "ok": False}
    client = None
    sampler = None
    began = time.monotonic()
    try:
        client = AcpClient(cmd, trace_dir, label, env=env, secrets=(key,))
        events = attach_timing(client)
        sampler = ResourceSampler(backend, client.proc.pid)
        sampler.start()
        t_spawn = time.monotonic()
        client.initialize()
        t_init = time.monotonic()
        new_session(client, session_cwd)
        t_session = time.monotonic()
        # configure() reads DSH_MODEL / DSH_REASONING_EFFORT from the harness env.
        client.configure(client.session_id)
        client.allow_permissions = True
        record.update({
            "spawn_to_initialize_s": round(t_init - t_spawn, 3),
            "spawn_to_session_s": round(t_session - t_spawn, 3),
            "initialize_rtt_ms": rpc_rtt(events, "initialize"),
            "session_new_rtt_ms": rpc_rtt(events, "session/new"),
        })
        turns, all_ok = [], True
        for step in (1, 2, 3):
            expected = task["reference"][f"step{step}_bytes"]
            cmd = command(mode, step, task["seed"]["burn"])
            row = None
            for attempt in (1, 2):
                prompt = prompt_for(index, step, cmd)
                if attempt == 2:
                    prompt += "\n上一步没有产生正确文件。请重新原样执行该命令，务必真正调用 shell 工具。"
                mark = len(events)
                t_turn = time.monotonic()
                try:
                    outcome = client.prompt(client.session_id, prompt, timeout=300)
                    timing = turn_timing(events, mark)
                    actual = {name: (workspace / name).read_bytes() if (workspace / name).exists() else None
                              for name in expected}
                    oracle_ok = all(actual[k] == v for k, v in expected.items()) and bool(expected)
                    row = {"step": step, "attempt": attempt, "turn_s": outcome["elapsed_s"],
                           "wall_s": round(time.monotonic() - t_turn, 3),
                           "tool_calls": outcome["tool_calls"], "updates": outcome["updates"],
                           "reply": outcome["reply"][:80], "oracle_ok": oracle_ok, **timing}
                except Exception as error:
                    row = {"step": step, "attempt": attempt, "oracle_ok": False,
                           "wall_s": round(time.monotonic() - t_turn, 3),
                           "error": f"{type(error).__name__}: {str(error)[:200]}"}
                    oracle_ok = False
                if oracle_ok:
                    break
            turns.append(row)
            all_ok = all_ok and row["oracle_ok"]
            print(f"  [{backend} {tid} step{step}] attempt={row['attempt']} "
                  f"turn={row.get('turn_s')}s tool={row.get('tool_span_s')}s "
                  f"oracle={row['oracle_ok']}", flush=True)
            if not row["oracle_ok"]:
                break
        record["turns"] = turns
        record["oracle_ok"] = all_ok
        record["ok"] = all_ok
        record["configure_rtt_ms"] = [rpc_rtt(events, "session/set_config_option")]
    except Exception as error:
        record["error"] = f"{type(error).__name__}: {str(error)[:400]}"
        try:
            record["stderr_tail"] = (trace_dir / f"{label}.stderr.log").read_text()[-800:]
        except Exception:
            pass
    finally:
        if sampler is not None:
            record["resources"] = sampler.stop()
        if client is not None:
            try:
                client.close(strict=False)
            except Exception:
                pass
        record["task_wall_s"] = round(time.monotonic() - began, 3)
        record["workspace_bytes"] = sum(p.stat().st_size for p in workspace.rglob("*") if p.is_file())
        record["workspace_files"] = sum(1 for p in workspace.rglob("*") if p.is_file())
    return record


def new_session(client, cwd):
    sid = client.new_session(cwd)
    client.session_id = sid
    return sid


# --------------------------------------------------------------------------
# Cold start measurement
# --------------------------------------------------------------------------
def cold_start(backend, key):
    label = f"cold-{backend}"
    workspace = WORKSPACES / f"cold-{backend}"
    workspace.mkdir(parents=True, exist_ok=True)
    (workspace / "input").mkdir(exist_ok=True)
    (workspace / "out").mkdir(exist_ok=True)
    if backend == "bare":
        home = RUNTIME / f"cold-bare-home-{int(time.time())}"
        home.mkdir(parents=True, exist_ok=True)
        env = base_env(key, dsh_home=home)
        cmd, cwd = make_command("bare", workspace, key)
    else:
        home = RUNTIME / f"cold-khome-{int(time.time())}"
        home.mkdir(parents=True, exist_ok=True)
        images = home / "images"
        if not images.exists():
            images.symlink_to(ROOT / ".release-test-3.3.0/home/images")
        env = base_env(key, k_home=home)
        cmd, cwd = make_command("kbox", workspace, key, k_home=home)
    began = time.monotonic()
    client = None
    try:
        client = AcpClient(cmd, TRACES, label, env=env, secrets=(key,))
        events = attach_timing(client)
        client.initialize()
        elapsed = time.monotonic() - began
        return {"backend": backend, "cold_spawn_to_initialize_s": round(elapsed, 3),
                "initialize_rtt_ms": rpc_rtt(events, "initialize"),
                "home": str(home), "ok": True}
    except Exception as error:
        return {"backend": backend, "ok": False, "error": f"{type(error).__name__}: {str(error)[:300]}",
                "elapsed_s": round(time.monotonic() - began, 3), "home": str(home)}
    finally:
        if client is not None:
            try:
                client.close(strict=False)
            except Exception:
                pass


# --------------------------------------------------------------------------
# Main
# --------------------------------------------------------------------------
def main():
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--pilot", action="store_true", help="run only T01 on both backends")
    ap.add_argument("--skip-cold", action="store_true")
    ap.add_argument("--backends", default="bare,kbox")
    args = ap.parse_args()

    os.environ["DSH_MODEL"] = "deepseek-flash"
    os.environ["DSH_REASONING_EFFORT"] = "low"
    for directory in (EVIDENCE, TRACES, WORKSPACES, RUNTIME):
        directory.mkdir(parents=True, exist_ok=True)
    key = credential()
    backends = args.backends.split(",")
    modes = MODES[:1] if args.pilot else MODES
    tasks = []
    for index, (tid, mode, name) in enumerate(modes):
        task = {"id": tid, "index": index, "mode": mode, "name": name, "seed": make_seed(index)}
        ref_dir = RUNTIME / f"ref-{tid}"
        if ref_dir.exists():
            import shutil
            shutil.rmtree(ref_dir)
        ref_dir.mkdir(parents=True)
        print(f"[reference] {tid} {mode} ...", flush=True)
        task["reference"] = run_reference(task, ref_dir)
        task["reference"][f"step{1}_ok"] = all(
            task["reference"][f"step{s}_rc"] == 0 for s in (1, 2, 3))
        # keep only digests of expected bytes for the manifest; full bytes stay in memory
        tasks.append(task)

    manifest = {"tasks": [{"id": t["id"], "mode": t["mode"], "name": t["name"],
                           "seed_sha256": hashlib.sha256(json.dumps(t["seed"], sort_keys=True).encode()).hexdigest(),
                           "burn": t["seed"]["burn"], "wait_ms": t["seed"]["wait_ms"]}
                          for t in tasks],
                "reference_compute_s": {t["id"]: [t["reference"][f"step{s}_s"] for s in (1, 2, 3)]
                                        for t in tasks},
                "reference_rc_ok": {t["id"]: t["reference"]["step1_ok"] for t in tasks}}
    (EVIDENCE / "tasks.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    cold = []
    if not args.skip_cold:
        for backend in backends:
            print(f"[cold] {backend} ...", flush=True)
            cold.append(cold_start(backend, key))
            print(json.dumps(cold[-1], ensure_ascii=False), flush=True)
        (EVIDENCE / "cold-start.json").write_text(json.dumps(cold, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    out_path = EVIDENCE / "samples.jsonl"
    with out_path.open("a", encoding="utf-8") as output:
        for task in tasks:
            # alternate A/B order to cancel temporal drift
            order = backends if task["index"] % 2 == 0 else list(reversed(backends))
            for backend in order:
                print(f"[run] {backend} {task['id']} {task['mode']} ...", flush=True)
                record = run_task(task, backend, key, "longrun")
                record["ts"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
                output.write(json.dumps(record, ensure_ascii=False) + "\n")
                output.flush()
                print(json.dumps({"task": task["id"], "backend": backend, "ok": record["ok"],
                                  "wall_s": record["task_wall_s"],
                                  "error": record.get("error")}, ensure_ascii=False), flush=True)
    print("DONE", out_path, flush=True)


if __name__ == "__main__":
    main()
