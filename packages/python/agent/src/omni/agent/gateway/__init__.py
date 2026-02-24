"""Gateway compatibility exports.

`create_webhook_app` is retained as a compatibility symbol but intentionally
raises at runtime because Python gateway loops are decommissioned.
"""

from .webhook import create_webhook_app

__all__ = ["create_webhook_app"]
