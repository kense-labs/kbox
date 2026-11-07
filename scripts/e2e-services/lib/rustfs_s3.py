#!/usr/bin/env python3
"""Minimal SigV4 S3 client for RustFS E2E (CreateBucket / PutObject / GetObject)."""
from __future__ import annotations

import hashlib
import hmac
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone
from email.utils import format_datetime
from urllib.parse import quote


def _sign(key: bytes, msg: str) -> bytes:
    return hmac.new(key, msg.encode(), hashlib.sha256).digest()


def _signing_key(secret: str, datestamp: str, region: str, service: str) -> bytes:
    k_date = _sign(("AWS4" + secret).encode(), datestamp)
    k_region = _sign(k_date, region)
    k_service = _sign(k_region, service)
    return _sign(k_service, "aws4_request")


def request(
    *,
    method: str,
    endpoint: str,
    path: str,
    access_key: str,
    secret_key: str,
    region: str = "us-east-1",
    payload: bytes = b"",
    content_type: str = "application/octet-stream",
    extra_headers: dict[str, str] | None = None,
) -> tuple[int, bytes]:
    # endpoint like http://127.0.0.1:9000
    if "://" not in endpoint:
        raise ValueError("endpoint must include scheme")
    host = endpoint.split("://", 1)[1]
    amz_date = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    datestamp = amz_date[:8]
    payload_hash = hashlib.sha256(payload).hexdigest()
    canonical_uri = quote(path if path.startswith("/") else "/" + path, safe="/-_.~")
    canonical_headers = (
        f"host:{host}\n"
        f"x-amz-content-sha256:{payload_hash}\n"
        f"x-amz-date:{amz_date}\n"
    )
    signed_headers = "host;x-amz-content-sha256;x-amz-date"
    canonical_request = "\n".join(
        [method, canonical_uri, "", canonical_headers, signed_headers, payload_hash]
    )
    credential_scope = f"{datestamp}/{region}/s3/aws4_request"
    string_to_sign = "\n".join(
        [
            "AWS4-HMAC-SHA256",
            amz_date,
            credential_scope,
            hashlib.sha256(canonical_request.encode()).hexdigest(),
        ]
    )
    signature = hmac.new(
        _signing_key(secret_key, datestamp, region, "s3"),
        string_to_sign.encode(),
        hashlib.sha256,
    ).hexdigest()
    authorization = (
        "AWS4-HMAC-SHA256 "
        f"Credential={access_key}/{credential_scope}, "
        f"SignedHeaders={signed_headers}, Signature={signature}"
    )
    headers = {
        "Host": host,
        "x-amz-date": amz_date,
        "x-amz-content-sha256": payload_hash,
        "Authorization": authorization,
    }
    if payload or method in {"PUT", "POST"}:
        headers["Content-Type"] = content_type
        headers["Content-Length"] = str(len(payload))
    if extra_headers:
        headers.update(extra_headers)
    url = endpoint.rstrip("/") + canonical_uri
    req = urllib.request.Request(url, data=payload if method != "GET" else None, method=method)
    for k, v in headers.items():
        req.add_header(k, v)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return resp.getcode(), resp.read()
    except urllib.error.HTTPError as err:
        return err.code, err.read()


def main() -> int:
    # rustfs_s3.py put|get|mb ENDPOINT AK SK BUCKET [KEY] [BODY]
    if len(sys.argv) < 6:
        print(
            "usage: rustfs_s3.py mb|put|get ENDPOINT AK SK BUCKET [KEY] [BODY]",
            file=sys.stderr,
        )
        return 2
    op, endpoint, ak, sk, bucket = sys.argv[1:6]
    if op == "mb":
        code, body = request(
            method="PUT",
            endpoint=endpoint,
            path=f"/{bucket}",
            access_key=ak,
            secret_key=sk,
            payload=b"",
            content_type="application/xml",
        )
        print(code)
        if code not in {200, 201, 204, 409}:  # 409 = already exists
            sys.stderr.write(body.decode(errors="replace"))
            return 1
        return 0
    if op == "put":
        key = sys.argv[6]
        payload = sys.argv[7].encode() if len(sys.argv) > 7 else b"kbox-e2e"
        code, body = request(
            method="PUT",
            endpoint=endpoint,
            path=f"/{bucket}/{key}",
            access_key=ak,
            secret_key=sk,
            payload=payload,
            content_type="text/plain",
        )
        print(code)
        if code not in {200, 201, 204}:
            sys.stderr.write(body.decode(errors="replace"))
            return 1
        return 0
    if op == "get":
        key = sys.argv[6]
        code, body = request(
            method="GET",
            endpoint=endpoint,
            path=f"/{bucket}/{key}",
            access_key=ak,
            secret_key=sk,
        )
        if code != 200:
            sys.stderr.write(body.decode(errors="replace"))
            print(code, file=sys.stderr)
            return 1
        sys.stdout.buffer.write(body)
        return 0
    print(f"unknown op {op}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
