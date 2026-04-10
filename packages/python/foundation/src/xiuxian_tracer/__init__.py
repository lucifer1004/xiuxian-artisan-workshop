"""Public tracer namespace with lazy exports."""

from __future__ import annotations

from importlib import import_module
from typing import Any

_EXPORTS: dict[str, tuple[str, str]] = {
    "CallbackManager": (".callbacks", "CallbackManager"),
    "CheckpointerRuntimeConfig": (".pipeline_schema", "CheckpointerRuntimeConfig"),
    "CompositeToolInvoker": (".composite_invoker", "CompositeToolInvoker"),
    "DispatchMode": (".async_utils", "DispatchMode"),
    "ExecutionStep": (".interfaces", "ExecutionStep"),
    "ExecutionTrace": (".interfaces", "ExecutionTrace"),
    "ExecutionTracer": (".engine", "ExecutionTracer"),
    "InMemoryTraceStorage": (".storage", "InMemoryTraceStorage"),
    "LoggingCallback": (".callbacks", "LoggingCallback"),
    "ToolClient": (".tool_invoker", "ToolClient"),
    "ToolClientInvoker": (".tool_invoker", "ToolClientInvoker"),
    "MappingToolInvoker": (".node_factory", "MappingToolInvoker"),
    "MemoryPool": (".interfaces", "MemoryPool"),
    "NoOpToolInvoker": (".node_factory", "NoOpToolInvoker"),
    "PipelineConfig": (".pipeline_schema", "PipelineConfig"),
    "PipelineExecutor": (".pipeline_runtime", "PipelineExecutor"),
    "PipelineRuntimeConfig": (".pipeline_schema", "PipelineRuntimeConfig"),
    "PipelineState": (".pipeline_schema", "PipelineState"),
    "PipelineWorkflowBuilder": (".pipeline_builder", "PipelineWorkflowBuilder"),
    "StateRuntimeConfig": (".pipeline_schema", "StateRuntimeConfig"),
    "StepType": (".interfaces", "StepType"),
    "ToolInvoker": (".node_factory", "ToolInvoker"),
    "TraceStorage": (".storage", "TraceStorage"),
    "TracedExecution": (".ui", "TracedExecution"),
    "TracerRuntimeConfig": (".pipeline_schema", "TracerRuntimeConfig"),
    "TracingCallback": (".callbacks", "TracingCallback"),
    "TracingCallbackHandler": (".workflow_events", "TracingCallbackHandler"),
    "compile_workflow": (".pipeline_checkpoint", "compile_workflow"),
    "console": (".ui", "console"),
    "create_default_invoker_stack": (".invoker_stack", "create_default_invoker_stack"),
    "create_in_memory_checkpointer": (
        ".pipeline_checkpoint",
        "create_in_memory_checkpointer",
    ),
    "create_pipeline_executor": (".pipeline_runtime", "create_pipeline_executor"),
    "create_pipeline_node": (".node_factory", "create_pipeline_node"),
    "create_traced_app": (".workflow_events", "create_traced_app"),
    "create_workflow_from_pipeline": (".pipeline_runtime", "create_workflow_from_pipeline"),
    "create_workflow_from_pipeline_with_defaults": (
        ".pipeline_runtime",
        "create_workflow_from_pipeline_with_defaults",
    ),
    "create_workflow_from_yaml": (".pipeline_runtime", "create_workflow_from_yaml"),
    "dispatch_coroutine": (".async_utils", "dispatch_coroutine"),
    "escape_xml": (".xml", "escape_xml"),
    "extract_attr": (".xml", "extract_attr"),
    "extract_tag": (".xml", "extract_tag"),
    "load_pipeline": (".pipeline_runtime", "load_pipeline"),
    "print_error": (".ui", "print_error"),
    "print_execution_path": (".ui", "print_execution_path"),
    "print_header": (".ui", "print_header"),
    "print_info": (".ui", "print_info"),
    "print_memory": (".ui", "print_memory"),
    "print_param": (".ui", "print_param"),
    "print_step_end": (".ui", "print_step_end"),
    "print_step_start": (".ui", "print_step_start"),
    "print_success": (".ui", "print_success"),
    "print_thinking": (".ui", "print_thinking"),
    "print_trace_summary": (".ui", "print_trace_summary"),
    "run_graphflow_pipeline": (".graphflow", "run_graphflow_pipeline"),
    "stream_with_trace": (".workflow_events", "stream_with_trace"),
    "traced": (".ui", "traced"),
    "traced_session": (".engine", "traced_session"),
}

__version__ = "0.2.0"
__all__ = sorted(_EXPORTS)


def __getattr__(name: str) -> Any:
    target = _EXPORTS.get(name)
    if target is None:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    module_name, attr_name = target
    module = import_module(module_name, package=__name__)
    value = getattr(module, attr_name)
    globals()[name] = value
    return value


def __dir__() -> list[str]:
    return sorted(set(globals()) | set(__all__))
