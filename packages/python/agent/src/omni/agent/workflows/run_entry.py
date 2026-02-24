"""CLI-facing reporting helpers and Omega mission entrypoints.

Python runtime loop entrypoints are decommissioned; orchestration is Rust-only.
"""

from __future__ import annotations

from typing import Any

from rich.box import ROUNDED
from rich.console import Console
from rich.markdown import Markdown
from rich.panel import Panel
from rich.table import Table

from omni.foundation.utils.common import setup_import_paths

setup_import_paths()

console = Console()


def print_banner() -> None:
    """Print CCA runtime banner."""
    from rich.text import Text

    banner = Text()
    banner.append(" CCA Runtime ", style="bold green")
    banner.append("• ", style="dim")
    banner.append("Omni Loop (v2.0)", style="italic")
    console.print(Panel(banner, expand=False, border_style="green"))


def print_session_report(
    task: str, result: dict, step_count: int, tool_counts: dict, tokens: int
) -> None:
    """Print enriched session report with stats."""
    grid = Table.grid(expand=True)
    grid.add_column()
    grid.add_row(f"[bold cyan]Task:[/bold cyan] {task}")
    grid.add_row(f"[bold dim]Session ID:[/bold dim] {result.get('session_id', 'N/A')}")
    grid.add_row("")
    metrics = Table(show_header=True, header_style="bold magenta", box=ROUNDED)
    metrics.add_column("Metric")
    metrics.add_column("Value", style="yellow")
    metrics.add_row("Steps", str(step_count))
    metrics.add_row("Tools", str(sum(tool_counts.values())))
    metrics.add_row("Est. Tokens", f"~{tokens}")
    grid.add_row(metrics)
    grid.add_row("")
    if tool_counts:
        t_table = Table(title="Tool Usage", show_header=False, box=ROUNDED)
        t_table.add_column("Tool")
        t_table.add_column("Count", justify="right")
        for tool, count in tool_counts.items():
            t_table.add_row(tool, f"[bold green]{count}[/bold green]")
        grid.add_row(t_table)
        grid.add_row("")
    grid.add_row("[bold green]Reflection & Outcome:[/bold green]")
    output = result.get("output", "Task completed")
    if output:
        output_str = str(output)
        if isinstance(output, dict):
            import json

            output_str = f"```json\n{json.dumps(output, indent=2)}\n```"
        import re

        output_str = re.sub(r"<thinking>.*?</thinking>", "", output_str, flags=re.DOTALL)
        output_str = output_str.strip()
        note_panel = Panel(Markdown(output_str), border_style="dim", expand=True)
        grid.add_row(note_panel)
    else:
        grid.add_row("Task completed")
    console.print(Panel(grid, title="✨ CCA Session Report ✨", border_style="green", expand=False))


async def execute_task_via_kernel(
    task: str, max_steps: int | None = None, verbose: bool = False
) -> dict[str, Any]:
    """Deprecated Python runtime loop entrypoint."""
    raise RuntimeError(
        "Python run_entry runtime is decommissioned. "
        "Use Rust runtime via `omni-agent` (e.g. `omni-agent repl` / `omni-agent gateway`)."
    )


async def execute_task_with_session(
    session_id: str,
    user_message: str,
    kernel: Any | None = None,
    max_steps: int = 20,
    verbose: bool = False,
    max_context_turns: int = 10,
    use_memory: bool = True,
) -> dict[str, Any]:
    """Deprecated Python runtime loop entrypoint with session persistence."""
    raise RuntimeError(
        "Python session runtime is decommissioned. "
        "Use Rust runtime via `omni-agent` (e.g. `omni-agent repl` / `omni-agent gateway`)."
    )


async def run_omega_mission(goal: str, socket_path: str):
    """Run Project Omega mission with TUI bridge. Returns mission result."""
    from omni.agent.cli.tui_bridge import TUIConfig, TUIManager
    from omni.agent.core.omni import MissionConfig, OmegaRunner

    config = TUIConfig(socket_path=socket_path, enabled=True)
    manager = TUIManager(config)
    async with manager.lifecycle() as bridge:
        mission_config = MissionConfig(
            goal=goal,
            enable_isolation=True,
            enable_conflict_detection=True,
            enable_memory_recall=True,
            enable_skill_crystallization=True,
            auto_merge=True,
            auto_recovery=True,
        )
        runner = OmegaRunner(config=mission_config, tui_bridge=bridge)
        return await runner.run_mission(goal)


def print_omega_result(result: Any, json_output: bool) -> None:
    """Print Omega mission result (table or JSON)."""
    if json_output:
        import json

        print(
            json.dumps(
                {
                    "success": result.success,
                    "duration_ms": result.duration_ms,
                    "tasks_completed": result.tasks_completed,
                    "tasks_failed": result.tasks_failed,
                    "conflicts_detected": result.conflicts_detected,
                },
                indent=2,
            )
        )
    else:
        table = Table(title="Ω Omega Mission Result", show_header=True)
        table.add_column("Metric", style="cyan")
        table.add_column("Value", style="yellow")
        table.add_row("Success", "✅ Yes" if result.success else "❌ No")
        table.add_row("Duration", f"{result.duration_ms:.0f}ms")
        table.add_row("Tasks", f"{result.tasks_completed}/{result.tasks_total}")
        table.add_row("Conflicts", str(result.conflicts_detected))
        console.print(table)


__all__ = [
    "console",
    "execute_task_via_kernel",
    "execute_task_with_session",
    "print_banner",
    "print_omega_result",
    "print_session_report",
    "run_omega_mission",
]
