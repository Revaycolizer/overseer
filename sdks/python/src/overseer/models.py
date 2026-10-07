"""Pydantic models for all Overseer REST API types."""

from datetime import datetime
from typing import Any, Optional

from pydantic import BaseModel, Field


# ── Auth ──────────────────────────────────────────────────────────────

class UserProfile(BaseModel):
    id: int
    username: str
    display_name: str


class AuthResponse(BaseModel):
    token: str
    user: UserProfile


# ── Conversations ─────────────────────────────────────────────────────

class Conversation(BaseModel):
    id: int
    title: str
    created_at: datetime


class ConversationSearchResult(BaseModel):
    conversation_id: int
    conversation_title: str
    message_id: int
    content: str
    created_at: datetime


# ── Messages & Attachments ────────────────────────────────────────────

class Message(BaseModel):
    id: int
    conversation_id: int
    role: str
    content: str
    created_at: datetime
    hide_evidence: bool
    attachments: list[Attachment] = Field(default_factory=list)


class Attachment(BaseModel):
    id: int
    message_id: int
    filename: str
    file_path: str
    content_type: str
    size: int
    extracted_content: Optional[str] = None


# ── Agents ────────────────────────────────────────────────────────────

class ToolInfo(BaseModel):
    id: int
    name: str
    description: str
    code: str
    language: str
    script_path: Optional[str] = None
    required_intent: Optional[str] = None
    safety_level: str
    version: str


class AgentInfo(BaseModel):
    id: int
    name: str
    description: str
    tools: list[str]
    instructions: str
    model: str
    domain: str
    version: str
    code: Optional[str] = None
    source_files: list[str]
    response_format: str


# ── Supervisors ───────────────────────────────────────────────────────

class Supervisor(BaseModel):
    id: int
    name: str
    description: Optional[str] = None
    instructions: Optional[str] = None
    model: Optional[str] = None
    domain: str
    version: Optional[str] = None
    code: Optional[str] = None
    bypass_synthesizer: bool
    fast_path_keywords: Optional[str] = None
    verification_agent_id: Optional[int] = None
    routing_priority: int
    hide_evidence: bool
    report_style: Optional[str] = None
    skip_fast_path_if_artifacts: bool
    artifact_fast_path_keywords: Optional[str] = None
    response_format: str


# ── Workflows ─────────────────────────────────────────────────────────

class Workflow(BaseModel):
    id: int
    name: str
    description: Optional[str] = None
    trigger_type: str
    trigger_config: Optional[str] = None
    status: str
    created_at: datetime
    updated_at: datetime


class WorkflowStep(BaseModel):
    id: int
    workflow_id: int
    step_order: int
    name: str
    prompt: str
    enabled: bool
    created_at: datetime
    updated_at: datetime


class WorkflowRun(BaseModel):
    id: int
    workflow_id: int
    conversation_id: Optional[int] = None
    status: str
    summary: Optional[str] = None
    error_message: Optional[str] = None
    created_at: datetime
    finished_at: Optional[datetime] = None


class WorkflowStepRun(BaseModel):
    id: int
    workflow_run_id: int
    workflow_step_id: int
    execution_run_id: Optional[int] = None
    step_order: int
    status: str
    output: Optional[str] = None
    error_message: Optional[str] = None
    created_at: datetime
    finished_at: Optional[datetime] = None


# ── Work Items ────────────────────────────────────────────────────────

class WorkItem(BaseModel):
    id: int
    conversation_id: int
    title: str
    brief: Optional[str] = None
    status: str
    surface_kind: str
    assigned_domain: Optional[str] = None
    prompt: str
    source_artifact_ids_json: Optional[str] = None
    metadata_json: Optional[str] = None
    created_at: datetime
    updated_at: datetime


class WorkItemEvent(BaseModel):
    id: int
    work_item_id: int
    execution_run_id: Optional[int] = None
    event_type: str
    status: Optional[str] = None
    summary: Optional[str] = None
    metadata_json: Optional[str] = None
    created_at: datetime


# ── Artifacts ─────────────────────────────────────────────────────────

class Artifact(BaseModel):
    id: int
    conversation_id: int
    execution_run_id: Optional[int] = None
    domain: Optional[str] = None
    step_text: Optional[str] = None
    kind: str
    label: str
    content: Optional[str] = None
    file_path: Optional[str] = None
    content_type: Optional[str] = None
    metadata_json: Optional[str] = None
    created_at: datetime


# ── Planners ──────────────────────────────────────────────────────────

class Planner(BaseModel):
    id: int
    name: str
    description: Optional[str] = None
    instructions: Optional[str] = None
    model: Optional[str] = None
    domain: str
    version: Optional[str] = None
    code: Optional[str] = None
    response_format: str


# ── Version Control ───────────────────────────────────────────────────

class EntityVersion(BaseModel):
    id: int
    version_tag: str
    changelog: Optional[str] = None
    created_at: datetime


# ── Health / System ───────────────────────────────────────────────────

class RuntimeHealth(BaseModel):
    ollama_reachable: bool
    ollama_endpoint: str
    available_models: list[str]
    selected_runtime_model: str
    runtime_model_available: bool
    title_model: str
    title_model_available: bool
    python_command: Optional[str] = None
    protoc_path: Optional[str] = None
    runtime_dir: str
    native_runtime: bool


class SystemLoad(BaseModel):
    label: str
    usage: int


class RuntimeRegistryStatus(BaseModel):
    status: str
    refreshed_at: Optional[str] = None
    error: Optional[str] = None


# ── Supervision / Computer Control ────────────────────────────────────

class ComputerSupervisionState(BaseModel):
    supervision_mode: str
    session_state: str
    session_note: str
    allowlist: list[str]
    max_actions: int
    action_count: int
    timeout_ms: int
    approval_state: str
    pending_tool: str
    pending_action: str
    pending_domain: str
    session_started_at: Optional[str] = None


# ── Credentials ───────────────────────────────────────────────────────

class CredentialSummary(BaseModel):
    id: int
    name: str
    cred_type: str = "api_key"


# ── Workflow Templates ────────────────────────────────────────────────

class WorkflowTemplate(BaseModel):
    id: int
    name: str
    description: Optional[str] = None
    category: str = "custom"


# ── Browser ───────────────────────────────────────────────────────────

class BrowserSession(BaseModel):
    id: int


class BrowserSessionInfo(BaseModel):
    id: Optional[int] = None
    session: Optional[BrowserSession] = None
    exists: Optional[bool] = None


# ── Generic ───────────────────────────────────────────────────────────

class ApiError(BaseModel):
    """Error returned by the API."""
    status: int
    body: str


class ServiceHealth(BaseModel):
    status: str
    version: str
    uptime_secs: int
    db_connected: bool
    ollama_reachable: bool
