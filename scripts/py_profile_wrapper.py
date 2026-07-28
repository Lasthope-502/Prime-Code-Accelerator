import cProfile
import pstats
import sys
import json

def main():
    if len(sys.argv) < 2:
        print("Usage: py_profile_wrapper.py <script.py> [args...]")
        sys.exit(1)

    script_path = sys.argv[1]
    script_args = sys.argv[2:]
    sys.argv = [script_path] + script_args

    profiler = cProfile.Profile()
    profiler.enable()

    try:
        with open(script_path) as f:
            code = compile(f.read(), script_path, 'exec')
        exec(code, {'__name__': '__main__', '__file__': script_path})
    finally:
        profiler.disable()

    stats = pstats.Stats(profiler)

    results = []
    for func, (cc, nc, tt, ct, callers) in stats.stats.items():
        filename, line, funcname = func
        results.append({
            "filename": filename,
            "line": line,
            "function": funcname,
            "calls": nc,
            "total_time": tt,
            "cumulative_time": ct,
        })

    results.sort(key=lambda x: x["cumulative_time"], reverse=True)

    with open("accel_py_profile.json", "w") as f:
        json.dump(results[:50], f, indent=2)

if __name__ == "__main__":
    main()