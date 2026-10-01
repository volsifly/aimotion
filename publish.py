#!/usr/bin/env python3
"""Publish one AI reply and its 16×16 motion sequence to the local viewer."""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path


ROOT = Path(__file__).resolve().parent
HOST = '127.0.0.1'
PORT = 8765
GRID_SIZE = 16
WORD = re.compile(r'^(?:0[xX])?[0-9a-fA-F]{4}$')


def validate(payload):
    if not isinstance(payload, dict):
        raise ValueError('输入必须是 JSON 对象。')
    text = payload.get('text')
    sequence = payload.get('sequence')
    if not isinstance(text, str):
        raise ValueError('text 必须是字符串。')
    if not isinstance(sequence, str):
        raise ValueError('sequence 必须是四位十六进制数序列。')
    words = [word for word in re.split(r'[\s,，;；|]+', sequence.strip()) if word]
    if not words or len(words) % GRID_SIZE or any(not WORD.fullmatch(word) for word in words):
        raise ValueError('sequence 每帧必须恰好包含 16 个四位十六进制数。')
    normalized = [' '.join(word.removeprefix('0x').removeprefix('0X').upper()
                           for word in words[index:index + GRID_SIZE])
                  for index in range(0, len(words), GRID_SIZE)]
    return {'id': uuid.uuid4().hex, 'size': GRID_SIZE, 'text': text,
            'sequence': '\n'.join(normalized)}, len(normalized)


def viewer_is_ready():
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        with opener.open(f'http://{HOST}:{PORT}/motion.js', timeout=0.5) as response:
            return b'AI Motion wire format' in response.read(1000)
    except (urllib.error.URLError, TimeoutError):
        return False


def ensure_viewer():
    if viewer_is_ready():
        return
    subprocess.Popen(
        [sys.executable, '-m', 'http.server', str(PORT), '--bind', HOST],
        cwd=ROOT, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL, start_new_session=True,
    )
    for _ in range(20):
        if viewer_is_ready():
            return
        time.sleep(0.1)
    raise RuntimeError(f'无法启动本地预览服务，或端口 {PORT} 已被其他服务占用。')


def publish(payload):
    with tempfile.NamedTemporaryFile('w', encoding='utf-8', dir=ROOT,
                                     prefix='.current-', suffix='.json', delete=False) as handle:
        temp_path = Path(handle.name)
        json.dump(payload, handle, ensure_ascii=False)
        handle.write('\n')
    try:
        os.replace(temp_path, ROOT / 'current.json')
    finally:
        temp_path.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('payload', nargs='?', default='-',
                        help='包含 text、sequence 的 JSON 文件；默认从标准输入读取')
    args = parser.parse_args()
    try:
        if args.payload == '-':
            source = json.load(sys.stdin)
        else:
            with open(args.payload, encoding='utf-8') as handle:
                source = json.load(handle)
        payload, frame_count = validate(source)
        ensure_viewer()
        publish(payload)
        print(json.dumps({'url': f'http://{HOST}:{PORT}/?reply={payload["id"]}',
                          'frames': frame_count}, ensure_ascii=False))
    except (OSError, ValueError, json.JSONDecodeError, RuntimeError) as error:
        print(f'发布失败：{error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
