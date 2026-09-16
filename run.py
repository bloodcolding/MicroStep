import argparse

from backend.server import main


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="MicroStep RPG local-first server")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--data", default=None, help="自定义事件流文件路径，默认 data/events.jsonl")
    args = parser.parse_args()
    main(args.host, args.port, args.data)
