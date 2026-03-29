"""Trace storage implementations for retained tracer surfaces."""

from __future__ import annotations

from abc import ABC, abstractmethod

from .interfaces import ExecutionTrace


class TraceStorage(ABC):
    """Abstract storage backend for execution traces."""

    @abstractmethod
    def save(self, trace: ExecutionTrace) -> None:
        """Persist an execution trace."""

    @abstractmethod
    def load(self, trace_id: str) -> ExecutionTrace | None:
        """Load a trace by identifier."""

    @abstractmethod
    def list_traces(self, *, limit: int | None = None) -> list[ExecutionTrace]:
        """List stored traces, newest first."""

    @abstractmethod
    def delete(self, trace_id: str) -> bool:
        """Delete a trace if present."""

    @abstractmethod
    def clear(self) -> None:
        """Delete all stored traces."""


class InMemoryTraceStorage(TraceStorage):
    """Ephemeral in-memory trace store used by tests and local tooling."""

    def __init__(self) -> None:
        self._traces: dict[str, ExecutionTrace] = {}

    def save(self, trace: ExecutionTrace) -> None:
        self._traces[trace.trace_id] = trace

    def load(self, trace_id: str) -> ExecutionTrace | None:
        return self._traces.get(trace_id)

    def list_traces(self, *, limit: int | None = None) -> list[ExecutionTrace]:
        traces = list(self._traces.values())
        traces.sort(key=lambda trace: trace.start_time, reverse=True)
        if limit is None:
            return traces
        return traces[:limit]

    def delete(self, trace_id: str) -> bool:
        return self._traces.pop(trace_id, None) is not None

    def clear(self) -> None:
        self._traces.clear()


__all__ = [
    "InMemoryTraceStorage",
    "TraceStorage",
]
