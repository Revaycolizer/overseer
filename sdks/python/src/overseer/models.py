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
    updated_at: datetime


class ConversationSearchResult(BaseModel):
    conversation_id: int
    message_id: int
    snippet: str


# ── Messages & Attachments ────────────────────────────────────────────

class Message(BaseModel):
    id: int
    conversation_id: int
    role: str
    content: str
    created_at: datetime
    hide_evidence: bool = False


class Attachment(BaseModel):
    id: int
    message_id: int
    filename: str
    file_path: str
    content_type: str
    size: int


# ── Agents ────────────────────────────────────────────────────────────

class ToolInfo(BaseModel):
    """Metadata about a registered tool."""
    id: int
    name: str
    description: str
    language: str
    code: str = ""
    version: str = "1.0"
    tool_type: str = ""
    domain: str = ""
    parameters_json: Optional[str] = None
    allowed_agents_json: Optional[str] = None
    safety_level: Optional[str] = None


class AgentInfo(BaseModel):
    """Information about a registered agent."""
    id: int
    name: str
    description: str
    domain: str
    tools: list[ToolInfo] = []
    instructions: str = ""
    model: str = ""
    version: str = "1.0"
    code: Optional[str] = None
    source_files: list[str] = []
    response_format: str = "json"


# ── Supervisors ───────────────────────────────────────────────────────

class Supervisor(BaseModel):
    id: int
    name: str
    description: str
    domain: str
    preamble: str = ""
    model: str = ""
    temperature: float = 0.7
    is_active: bool = True
    agents: list[AgentInfo] = []


# ── Workflows ─────────────────────────────────────────────────────────

class Workflow(BaseModel):
    id: int
    name: str
    description: Optional[str] = None
    status: str = "draft"
    created_at: datetime
    updated_at: datetime


class WorkflowStep(BaseModel):
    id: int
    workflow_id: int
    step_order: int = 0
    name: str = ""
    tool_name: Optional[str] = None
    config_json: Optional[str] = None


class WorkflowRun(BaseModel):
    id: int
    workflow_id: int
    status: str
    started_at: datetime
    finished_at: Optional[datetime] = None
    result_summary: Optional[str] = None


class WorkflowStepRun(BaseModel):
    id: int
    run_id: int
    step_name: str
    status: str
    output_json: Optional[str] = None
    started_at: datetime
    finished_at: Optional[datetime] = None


# ── Work Items ────────────────────────────────────────────────────────

class WorkItem(BaseModel):
    id: int
    conversation_id: int
    title: str
    status: str
    surface_kind: str = "chat"
    created_at: datetime
    updated_at: datetime


class WorkItemEvent(BaseModel):
    id: int
    work_item_id: int
    execution_run_id: Optional[int] = None
    event_type: str
    status: Optional[str] = None
    summary: Optional[str] = None
    created_at: datetime


# ── Artifacts ─────────────────────────────────────────────────────────

class Artifact(BaseModel):
    id: int
    conversation_id: int
    execution_run_id: Optional[int] = None
    kind: str
    label: str
    content: Optional[str] = None
    file_path: Optional[str] = None
    content_type: Optional[str] = None
    created_at: datetime


# ── Planners ──────────────────────────────────────────────────────────

class Planner(BaseModel):
    id: int
    name: str
    instructions: str
    model: str = ""
    is_active: bool = True


# ── Version Control ───────────────────────────────────────────────────

class EntityVersion(BaseModel):
    id: int
    entity_type: str
    entity_id: int
    tag: str
    created_at: datetime
    description: Optional[str] = None


# ── Health / System ───────────────────────────────────────────────────

class RuntimeHealth(BaseModel):
    status: str
    version: str
    uptime_secs: int
    db_connected: bool
    ollama_reachable: bool


class SystemLoad(BaseModel):
    cpu_percent: float
    memory_percent: float
    agent_count: int
    active_conversations: int


class RuntimeRegistryStatus(BaseModel):
    last_refresh: Optional[datetime] = None
    agent_count: int = 0
    tool_count: int = 0


# ── Supervision / Computer Control ────────────────────────────────────

class ComputerSupervisionState(BaseModel):
    enabled: bool
    allowlist: str = ""
    pending_action: Optional[dict[str, Any]] = None
    recent_executions: list[str] = []


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
    session: Optional[BrowserSession] = None
    exists: Optional[bool] = None


# ── Generic ───────────────────────────────────────────────────────────

class ApiError(BaseModel):
    """Error returned by the API."""
    status: int
    body: str
