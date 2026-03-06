"""
mcp.py - MCP Server Command

High-performance MCP Server using omni.mcp transport layer.

Usage:
    omni mcp --transport stdio     # Claude Desktop (default)
    omni mcp --transport sse --port 3000  # Claude Code CLI / debugging
"""

from __future__ import annotations

import asyncio
import code
import os
import signal
import sys
import time
from concurrent.futures import TimeoutError as FutureTimeoutError
from enum import Enum
from typing import TYPE_CHECKING, Annotated, Any

# Python 3.13 removed code.InteractiveConsole, but torch.distributed imports pdb
# which references it during module load. Keep a minimal shim.
if not hasattr(code, "InteractiveConsole"):

    class _DummyInteractiveConsole:
        def __init__(self, *args, **kwargs):
            pass

    code.InteractiveConsole = _DummyInteractiveConsole

os.environ.setdefault("TORCH_DISTRIBUTED_DETECTION", "1")

import typer
from rich.panel import Panel

from omni.agent.mcp_server.startup import (
    initialize_handler_on_server_loop as _initialize_handler_on_server_loop,
)
from omni.agent.mcp_server.startup import (
    wait_for_sse_server_readiness as _wait_for_sse_server_readiness,
)
from omni.foundation.config.logging import configure_logging, get_logger
from omni.foundation.utils.asyncio import run_async_blocking

from ..console import err_console

if TYPE_CHECKING:
    from omni.agent.server import AgentMCPHandler


def _is_transient_embedding_warm_error(error: Exception) -> bool:
    message = str(error).lower()
    transient_markers = (
        "apiconnectionerror",
        "server disconnected without sending a response",
        "connection refused",
        "failed to connect",
        "remoteprotocolerror",
        "temporarily unavailable",
        "read timeout",
        "connect timeout",
    )
    return any(marker in message for marker in transient_markers)


async def _warm_embedding_after_startup(
    timeout_seconds: float = 8.0,
    *,
    max_attempts: int = 8,
    retry_delay_seconds: float = 0.3,
) -> None:
    """Warm embedding backend with bounded timeout and transient-connection retries."""
    logger = get_logger("omni.mcp.embedding")
    try:
        from omni.foundation.services.embedding import get_embedding_service

        embed_svc = get_embedding_service()
        if embed_svc.backend == "unavailable":
            return

        loop = asyncio.get_running_loop()
        deadline = loop.time() + max(timeout_seconds, 0.1)
        attempts = 0
        last_error: Exception | None = None

        while attempts < max(1, max_attempts):
            attempts += 1
            remaining = deadline - loop.time()
            if remaining <= 0:
                break
            try:
                await asyncio.wait_for(
                    loop.run_in_executor(None, lambda: embed_svc.embed("_warm_")),
                    timeout=remaining,
                )
                logger.info(
                    "Embedding backend warmed for fast first request (attempt=%s/%s)",
                    attempts,
                    max(1, max_attempts),
                )
                return
            except TimeoutError:
                logger.warning(
                    "Embedding warm timed out after %.1fs; continue startup", timeout_seconds
                )
                return
            except Exception as error:
                last_error = error
                if not _is_transient_embedding_warm_error(error):
                    logger.warning("Embedding warm skipped: %s", error)
                    return
                if attempts >= max(1, max_attempts):
                    break
                logger.info(
                    "Embedding warm transient failure; retrying (attempt=%s/%s): %s",
                    attempts,
                    max(1, max_attempts),
                    error,
                )
                sleep_seconds = min(max(retry_delay_seconds, 0.0), max(0.0, deadline - loop.time()))
                if sleep_seconds > 0:
                    await asyncio.sleep(sleep_seconds)

        if last_error is not None:
            logger.warning(
                "Embedding warm skipped after %s attempts: %s",
                attempts,
                last_error,
            )
    except TimeoutError:
        logger.warning("Embedding warm timed out after %.1fs; continue startup", timeout_seconds)
    except Exception as e:
        logger.warning("Embedding warm skipped: %s", e)


# =============================================================================
# MCP Session Handler for SSE Transport
# =============================================================================


async def _run_mcp_session(
    handler: AgentMCPHandler,
    read_stream: Any,
    write_stream: Any,
) -> None:
    """Run MCP session by processing messages from read_stream and writing to write_stream.

    This bridges the SSE transport streams with the AgentMCPHandler.
    """
    import anyio

    logger = get_logger("omni.mcp.session")

    async def read_messages():
        """Read messages from the read_stream and process them."""
        try:
            async for session_message in read_stream:
                # SessionMessage contains the MCP message
                message = session_message.message
                logger.debug(f"Received MCP message: {message.method}")

                # Handle the message using handler
                if hasattr(message, "id") and message.id is not None:
                    # It's a request (expects response)
                    request_dict = message.model_dump(by_alias=True, exclude_none=True)
                    response = await handler.handle_request(request_dict)
                    # Send response back
                    await write_stream.send(session_message.response(response))
                else:
                    # It's a notification (no response expected)
                    await handler.handle_notification(
                        message.method,
                        message.params.model_dump(by_alias=True) if message.params else None,
                    )
        except anyio.BrokenResourceError:
            logger.info("SSE session closed")
        except Exception as e:
            logger.error(f"Error in MCP session: {e}")

    # Run the message processing task
    await read_messages()


# Transport mode enumeration
class TransportMode(str, Enum):
    stdio = "stdio"  # Production mode (Claude Desktop)
    sse = "sse"  # Development/debug mode (Claude Code CLI)


# Global for graceful shutdown
_shutdown_requested = False
_shutdown_count = 0  # For SSE mode signal handling
_handler_ref = None
_transport_ref = None  # For stdio transport stop
_server_loop_ref: asyncio.AbstractEventLoop | None = None


# =============================================================================
# Simple signal handler for stdio mode - mimics old stdio.py behavior
# =============================================================================

_stdio_shutdown_count = 0


def _setup_stdio_signal_handler() -> None:
    """Set up signal handler for stdio mode (simple approach)."""
    import sys as _sys

    def signal_handler(*_args):
        global _stdio_shutdown_count
        _stdio_shutdown_count += 1
        _sys.stderr.write(f"\n[CLI] Signal received! Count: {_stdio_shutdown_count}\n")
        _sys.stderr.flush()
        if _stdio_shutdown_count == 1:
            _sys.exit(0)  # Normal exit
        else:
            import os as _os

            _os._exit(1)  # Force exit on second Ctrl-C

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)
    _sys.stderr.write("[CLI] Signal handler registered\n")
    _sys.stderr.flush()


def _setup_signal_handler(handler_ref=None, transport_ref=None, stdio_mode=False) -> None:
    """Setup signal handlers for graceful shutdown."""
    global _shutdown_count

    def signal_handler(signum, frame):
        global _shutdown_requested, _shutdown_count
        _shutdown_requested = True
        _shutdown_count += 1

        if stdio_mode:
            # In stdio mode: first Ctrl-C = graceful exit, second = force exit
            import os as _os
            import sys as _sys

            try:
                if _shutdown_count == 1:
                    _sys.stderr.write("\n[CLI] Shutdown signal received, exiting...\n")
                    _sys.stderr.flush()
                    sys.exit(0)  # Allow graceful shutdown
                else:
                    _os._exit(1)  # Force exit on second Ctrl-C
            except Exception:
                _os._exit(1)

        # SSE mode: stop the transport first (breaks the run_loop)
        if transport_ref is not None:
            _stop_transport_for_shutdown(transport_ref)

        _sync_graceful_shutdown()

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)


async def _graceful_shutdown(handler) -> None:
    """Perform graceful shutdown of kernel and server."""
    logger = get_logger("omni.mcp.shutdown")

    try:
        # Shutdown kernel gracefully
        if hasattr(handler, "_kernel") and handler._kernel is not None:
            kernel = handler._kernel
            if kernel.is_ready or kernel.state.value in ("ready", "running"):
                logger.info("🛑 Initiating graceful shutdown...")
                await kernel.shutdown()
                logger.info("✅ Kernel shutdown complete")

    except Exception as e:
        logger.error(f"Error during shutdown: {e}")


def _run_coroutine_for_shutdown(
    coro: Any,
    *,
    timeout_seconds: float,
    action: str,
) -> None:
    """Run shutdown coroutine on the SSE server loop when available."""
    loop = _server_loop_ref
    if loop is not None and loop.is_running() and not loop.is_closed():
        future = asyncio.run_coroutine_threadsafe(coro, loop)
        try:
            future.result(timeout=timeout_seconds)
            return
        except FutureTimeoutError as e:
            future.cancel()
            raise TimeoutError(f"{action} timed out after {timeout_seconds}s") from e
    run_async_blocking(coro)


def _stop_transport_for_shutdown(transport_ref, *, timeout_seconds: float = 10.0) -> None:
    """Stop transport during shutdown without crossing event loops."""
    logger = get_logger("omni.mcp.shutdown")
    try:
        _run_coroutine_for_shutdown(
            transport_ref.stop(),
            timeout_seconds=timeout_seconds,
            action="transport stop",
        )
    except Exception as e:
        logger.warning("Transport stop failed during shutdown: %s", e)


def _sync_graceful_shutdown() -> None:
    """Sync wrapper for graceful shutdown (for signal handler)."""
    global _handler_ref
    logger = get_logger("omni.mcp.shutdown")
    if _handler_ref is not None:
        try:
            _run_coroutine_for_shutdown(
                _graceful_shutdown(_handler_ref),
                timeout_seconds=30.0,
                action="graceful shutdown",
            )
        except Exception as e:
            logger.error("Graceful shutdown failed: %s", e)


def register_mcp_command(app_instance: typer.Typer) -> None:
    """Register mcp command directly with the main app."""
    from omni.agent.cli.load_requirements import register_requirements

    register_requirements("mcp", embedding_index=True)

    @app_instance.command("mcp", help="Start Omni MCP Server (Level 2 Transport)")
    def run_mcp(
        transport: Annotated[
            TransportMode,
            typer.Option(
                "--transport",
                "-t",
                help="Communication transport mode (stdio for Claude Desktop, sse for Claude Code CLI)",
            ),
        ] = TransportMode.sse,
        host: Annotated[
            str,
            typer.Option(
                "--host",
                "-h",
                help="Host to bind to (SSE only, 127.0.0.1 for local security)",
            ),
        ] = "127.0.0.1",
        port: Annotated[
            int,
            typer.Option(
                "--port",
                "-p",
                help="Port to listen on (only for SSE mode, use 0 for random)",
            ),
        ] = 3000,
        verbose: Annotated[
            bool,
            typer.Option(
                "--verbose",
                "-v",
                help="Enable verbose mode (hot reload, debug logging)",
            ),
        ] = False,
        no_embedding: Annotated[
            bool,
            typer.Option(
                "--no-embedding",
                help="Skip embedding service (lightweight mode; knowledge.recall will fail)",
            ),
        ] = False,
    ):
        """
        Start Omni MCP Server with high-performance omni.mcp transport layer.

        Uses Rust-powered orjson for 10-50x faster JSON serialization.
        """
        global _handler_ref, _transport_ref, _server_loop_ref

        try:
            if transport == TransportMode.stdio:
                # Configure logging (stdout is used by MCP, so log to stderr)
                log_level = "DEBUG" if verbose else "INFO"
                configure_logging(level=log_level)
                logger = get_logger("omni.mcp.stdio")

                async def run_stdio():
                    """Run stdio mode."""
                    logger.info("📡 Starting Omni MCP Server (STDIO mode)")

                    if not no_embedding:
                        # MCP must not load embedding models in-process.
                        # Use client-only: connect to an existing embedding service.
                        os.environ["OMNI_EMBEDDING_CLIENT_ONLY"] = "1"
                        from omni.foundation.services.embedding import get_embedding_service

                        embed_svc = get_embedding_service()
                        embed_svc.initialize()
                        if embed_svc.backend == "http":
                            logger.info("✅ Embedding: client mode (using existing service)")
                        elif embed_svc.backend == "unavailable":
                            logger.warning(
                                "Embedding service unreachable; cortex indexing will be skipped. "
                                "Start the Rust embedding service "
                                "(GET /health, POST /embed/single)."
                            )
                        else:
                            logger.info("✅ Embedding: %s mode", embed_svc.backend)
                        if embed_svc.backend != "unavailable":
                            await _warm_embedding_after_startup()
                    else:
                        logger.info("⏭️ Embedding service skipped (--no-embedding)")

                    # Run stdio server (it handles its own server/handler creation)
                    from omni.agent.mcp_server.stdio import run_stdio as old_run_stdio

                    await old_run_stdio(verbose=verbose)

                run_async_blocking(run_stdio())

            else:  # SSE mode - uses sse.py module
                # Configure logging
                log_level = "DEBUG" if verbose else "INFO"
                configure_logging(level=log_level)
                logger = get_logger("omni.mcp.sse")

                err_console.print(
                    Panel(
                        f"[bold green]🚀 Starting Omni MCP in {transport.value.upper()} mode on port {port}[/bold green]"
                        + (" [cyan](verbose, hot-reload enabled)[/cyan]" if verbose else ""),
                        style="green",
                    )
                )

                # Create handler (lightweight, no initialization yet)
                from omni.agent.server import create_agent_handler

                handler = create_agent_handler()
                handler.set_verbose(verbose)
                _handler_ref = handler

                # Import SSE server
                # Start SSE server FIRST (so MCP clients can connect immediately)
                # Use threading to run server in background while we initialize services
                import threading

                from omni.agent.mcp_server.sse import run_sse

                server_loop_ready = threading.Event()
                server_loop_holder: dict[str, asyncio.AbstractEventLoop] = {}
                server_error = [None]

                def run_server():
                    global _server_loop_ref
                    loop = asyncio.new_event_loop()
                    try:
                        asyncio.set_event_loop(loop)
                        _server_loop_ref = loop
                        server_loop_holder["loop"] = loop
                        server_loop_ready.set()
                        loop.run_until_complete(run_sse(handler, host, port))
                    except Exception as e:
                        server_error[0] = e
                    finally:
                        if _server_loop_ref is loop:
                            _server_loop_ref = None
                        server_loop_ready.set()
                        if not loop.is_closed():
                            loop.close()

                server_thread = threading.Thread(target=run_server, daemon=True)
                server_thread.start()
                server_loop_ready.wait(timeout=2.0)

                # Wait for server readiness and fail fast if startup failed.
                _wait_for_sse_server_readiness(host, port, server_thread, server_error)

                logger.info(f"✅ SSE server started on http://{host}:{port}")

                # Initialize handler first so MCP initialize/tool discovery can respond quickly.
                server_loop = server_loop_holder.get("loop")
                init_started = time.perf_counter()
                if server_loop is not None and server_loop.is_running():
                    _initialize_handler_on_server_loop(handler, server_loop, timeout_seconds=90.0)
                else:
                    logger.warning(
                        "SSE server loop unavailable for handler init; falling back to temporary loop"
                    )
                    run_async_blocking(handler.initialize())
                logger.info(
                    "✅ MCP handler initialized (init_ms=%.1f)",
                    (time.perf_counter() - init_started) * 1000.0,
                )

                # Initialize embedding services after MCP handler is ready.
                if not no_embedding:
                    # MCP must not load embedding models in-process.
                    os.environ["OMNI_EMBEDDING_CLIENT_ONLY"] = "1"
                    from omni.foundation.services.embedding import get_embedding_service

                    embed_svc = get_embedding_service()
                    embed_svc.initialize()
                    if embed_svc.backend == "http":
                        logger.info("✅ Embedding: client mode (using existing service)")
                    elif embed_svc.backend == "unavailable":
                        logger.warning(
                            "Embedding service unreachable; cortex indexing will be skipped. "
                            "Start the Rust embedding service "
                            "(GET /health, POST /embed/single)."
                        )
                    else:
                        logger.info("✅ Embedding: %s mode", embed_svc.backend)
                    if embed_svc.backend != "unavailable":
                        run_async_blocking(_warm_embedding_after_startup())
                else:
                    logger.info("⏭️ Embedding service skipped (--no-embedding)")

                # Keep main thread alive until server thread exits (e.g. Ctrl+C)
                try:
                    server_thread.join()
                except KeyboardInterrupt:
                    logger.info("Server stopped")

                # Server thread exited; run graceful shutdown and exit (do not start server again)
                shutdown_logger = get_logger("omni.mcp.shutdown")
                shutdown_logger.info("👋 Shutting down...")
                _sync_graceful_shutdown()
                sys.exit(0)

        except KeyboardInterrupt:
            shutdown_logger = get_logger("omni.mcp.shutdown")
            shutdown_logger.info("👋 Server interrupted by user")
            if _handler_ref is not None:
                _sync_graceful_shutdown()
            sys.exit(0)
        except Exception as e:
            err_console.print(Panel(f"[bold red]Server Error:[/bold red] {e}", style="red"))
            if _handler_ref is not None:
                _sync_graceful_shutdown()
            sys.exit(1)


__all__ = ["register_mcp_command"]
