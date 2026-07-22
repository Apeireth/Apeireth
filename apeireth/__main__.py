"""Allow ``python -m apeireth`` to use the public CLI."""
from .cli import main


if __name__ == "__main__":
    raise SystemExit(main())
