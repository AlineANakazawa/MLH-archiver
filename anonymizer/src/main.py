"""MLH Anonymizer - Entry point.

Pseudo-anonymize personal identification data in mailing list datasets.
"""

import os
import logging
import signal
import time
from datetime import datetime
from multiprocessing import get_context
import subprocess

from mlh_anonymizer.configs import (
    N_PROC,
    LISTS_TO_PARSE,
    DEBUG,
    INPUT_DIR_PATH,
    OUTPUT_DIR_PATH,
)
from mlh_anonymizer.list_processor import parse_mail_at

status_path = f"{OUTPUT_DIR_PATH}/__status/"

# Configure logging
level = logging.INFO
if DEBUG:
    level = logging.DEBUG

logging.basicConfig(
    level=level,
    format="[%(asctime)s] {%(pathname)s:%(lineno)d} %(levelname)s - %(message)s",
    datefmt="%H:%M:%S",
)

logger = logging.getLogger(__name__)


def parse_mail_at_wrap(mailing_list: str) -> None:
    """Wrapper for parse_mail_at with fixed paths."""
    status = open(
        f"{status_path}/{mailing_list}.log",
        "w",
        encoding="utf-8",
    )

    try:
        status.write(f"Starting {mailing_list} at {datetime.now().isoformat()}\n")
        status.flush()
        parse_mail_at(mailing_list, INPUT_DIR_PATH, OUTPUT_DIR_PATH)
        status.write(f"Completed {mailing_list} at {datetime.now().isoformat()}\n")
        status.flush()
    except Exception as e:
        status.write(f"Failed {mailing_list} at {datetime.now().isoformat()}:  {e}\n")
        status.flush()
        raise e


def main() -> None:
    logging.info("anonymizer starting — build: %s", get_build_info())
    """Main entry point for the anonymizer."""
    # Parse specific lists or all in the directory
    lists = LISTS_TO_PARSE if len(LISTS_TO_PARSE) > 0 else os.listdir(INPUT_DIR_PATH)

    os.makedirs(OUTPUT_DIR_PATH, exist_ok=True)
    os.makedirs(status_path, exist_ok=True)

    if N_PROC == 1:
        sequential(lists)
    else:
        run_parallel(lists)


def kill_pool_workers(pool) -> None:
    for p in pool._pool:
        if p.is_alive():
            try:
                os.kill(p.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass


def join_pool_with_escalation(pool, timeout: float = 3.0) -> None:
    deadline = time.monotonic() + timeout
    workers = list(pool._pool)
    for p in workers:
        remaining = max(0, deadline - time.monotonic())
        p.join(timeout=remaining)
    alive = [p for p in workers if p.is_alive()]
    if not alive:
        return
    logging.info(
        "%d workers still alive after %.1fs, sending SIGKILL...", len(alive), timeout
    )
    kill_pool_workers(pool)


def run_parallel(lists: list[str]) -> None:
    pool = get_context("spawn").Pool(N_PROC)
    interrupted = False

    def handle_signal(signum: int, frame: object) -> None:
        nonlocal interrupted
        if not interrupted:
            interrupted = True
            logging.info("Received signal %s — terminating workers.", signum)
            pool.terminate()
        else:
            logging.info("Second signal %s — force-killing workers.", signum)
            kill_pool_workers(pool)
            os._exit(1)

    original_sigint = signal.signal(signal.SIGINT, handle_signal)
    original_sigterm = signal.signal(signal.SIGTERM, handle_signal)

    try:
        for _ in pool.imap_unordered(parse_mail_at_wrap, lists):
            pass
    except KeyboardInterrupt:
        logging.info("Interrupted, terminating workers...")
        pool.terminate()
    finally:
        pool.close()
        join_pool_with_escalation(pool)
        signal.signal(signal.SIGINT, original_sigint)
        signal.signal(signal.SIGTERM, original_sigterm)
        logging.info("All workers shut down.")


def get_build_info() -> str:
    """Get build commit info from container env, or fall back to local git."""
    commit = os.getenv("BUILD_GIT_COMMIT")
    date = os.getenv("BUILD_GIT_DATE")

    if commit and commit != "unknown":
        return f"commit {commit} ({date})"

    try:
        commit = subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=os.path.dirname(os.path.abspath(__file__)),
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        date = subprocess.check_output(
            ["git", "log", "-1", "--format=%ci"],
            cwd=os.path.dirname(os.path.abspath(__file__)),
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        return f"commit {commit} ({date})"
    except Exception:
        return "unknown"


def sequential(lists: list) -> None:
    """Run anonymization sequentially (for debugging).

    Args:
        lists: List of mailing list names to process
    """
    for mailing_list in lists:
        parse_mail_at_wrap(mailing_list)


if __name__ == "__main__":
    main()
