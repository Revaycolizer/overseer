"""HTTP client for the Overseer REST API."""

import json
from typing import Any, Optional
from urllib.parse import urlencode

import httpx

from .auth import AuthManager
from .chat import ChatAPI
from urllib.parse import urlsplit
import math
from .models import (
    AgentInfo,
    Artifact,
    Attachment,
    AuthResponse,
    BrowserSessionInfo,
    ComputerSupervisionState,
    Conversation,
    ConversationSearchResult,
    CredentialSummary,
    EntityVersion,
    Message,
    Planner,
    RuntimeHealth,
    ServiceHealth,
    RuntimeRegistryStatus,
    Supervisor,
    SystemLoad,
    ToolInfo,
    UserProfile,
    WorkItem,
    WorkItemEvent,
    Workflow,
    WorkflowRun,
    WorkflowStep,
    WorkflowStepRun,
    WorkflowTemplate,
)


class OverseerClient:
    """Async HTTP client for the Overseer AI orchestration platform.

    Usage:
        async with OverseerClient("http://localhost:8080", token="...") as client:
            conversations = await client.conversations.list()
            agents = await client.agents.list()
    """

    def __init__(
        self,
        base_url: str = "http://localhost:8080",
        token: Optional[str] = None,
        timeout: float = 30.0,
        *, transport: Optional[httpx.AsyncBaseTransport] = None,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        parsed = urlsplit(self.base_url)
        if parsed.scheme not in ("http", "https") or not parsed.hostname or parsed.username or parsed.password or parsed.path or parsed.query or parsed.fragment:
            raise ValueError("base_url must be an HTTP(S) server origin without credentials")
        if not math.isfinite(timeout) or timeout <= 0:
            raise ValueError("timeout must be positive")
        self._auth = AuthManager()
        if token:
            self._auth.token = token
        self._http = httpx.AsyncClient(
            base_url=self.base_url,
            timeout=httpx.Timeout(timeout),
            headers=self._default_headers(),
            transport=transport,
            follow_redirects=False,
        )
        # Namespaced API modules
        self.chat = ChatAPI(self.base_url, lambda: self.token)
        self.conversations = _ConversationsAPI(self)
        self.messages = _MessagesAPI(self)
        self.agents = _AgentsAPI(self)
        self.tools = _ToolsAPI(self)
        self.supervisors = _SupervisorsAPI(self)
        self.workflows = _WorkflowsAPI(self)
        self.work_items = _WorkItemsAPI(self)
        self.artifacts = _ArtifactsAPI(self)
        self.planners = _PlannersAPI(self)
        self.supervision = _SupervisionAPI(self)
        self.versions = _VersionsAPI(self)
        self.browser = _BrowserAPI(self)
        self.credentials = _CredentialsAPI(self)
        self.templates = _TemplatesAPI(self)
        self.settings = _SettingsAPI(self)
        self.system = _SystemAPI(self)

    def _default_headers(self) -> dict[str, str]:
        return {
            "User-Agent": "overseer-client-py/0.2.0",
            "Accept": "application/json",
        }

    @property
    def token(self) -> Optional[str]:
        return self._auth.token

    @token.setter
    def token(self, value: Optional[str]) -> None:
        self._auth.token = value

    @property
    def auth_header(self) -> dict[str, str]:
        return self._auth.auth_header

    async def __aenter__(self) -> "OverseerClient":
        return self

    async def __aexit__(self, *args: Any) -> None:
        await self._http.aclose()

    async def close(self) -> None:
        await self._http.aclose()

    # ── Low-level HTTP helpers ─────────────────────────────────────────

    async def _request(
        self,
        method: str,
        path: str,
        params: Optional[dict[str, Any]] = None,
        json_body: Any = None,
    ) -> httpx.Response:
        if not path.startswith("/") or path.startswith("//"):
            raise ValueError("path must be relative to the configured server")
        url = path
        if params:
            url = f"{path}?{urlencode({k: str(v) for k, v in params.items() if v is not None})}"
        headers = self.auth_header
        r = await self._http.request(method, url, json=json_body, headers=headers)
        if not r.is_success:
            body = r.text[:500]
            raise httpx.HTTPStatusError(
                f"API error {r.status_code}: {body}",
                request=r.request,
                response=r,
            )
        return r

    async def _get(self, path: str, params: Optional[dict] = None) -> httpx.Response:
        return await self._request("GET", path, params=params)

    async def _post(self, path: str, body: Any = None) -> httpx.Response:
        return await self._request("POST", path, json_body=body)

    async def _patch(self, path: str, body: Any = None) -> httpx.Response:
        return await self._request("PATCH", path, json_body=body)

    async def _delete(self, path: str) -> httpx.Response:
        return await self._request("DELETE", path)

    async def _get_json(self, path: str, params: Optional[dict] = None, model: Optional[type] = None) -> Any:
        r = await self._get(path, params=params)
        data = r.json() if r.content else None
        if model and isinstance(data, list):
            return [model(**item) for item in data]
        if model and isinstance(data, dict):
            return model(**data)
        return data

    async def _post_json(self, path: str, body: Any = None, model: Optional[type] = None) -> Any:
        r = await self._post(path, body)
        data = r.json() if r.content else None
        if model and isinstance(data, list):
            return [model(**item) for item in data]
        if model and isinstance(data, dict):
            return model(**data)
        return data

    async def request(self, method: str, path: str, *, body: Any = None, params: Optional[dict[str, Any]] = None) -> Any:
        """Call an additional JSON endpoint on the configured server."""
        response = await self._request(method, path, params=params, json_body=body)
        return response.json() if response.content else None

    # ── Auth ───────────────────────────────────────────────────────────

    async def login(self, username: str, password: str) -> str:
        """Login and return a bearer token."""
        r = await self._post("/v1/auth/login", {"username": username, "password": password})
        data = AuthResponse(**r.json())
        self._auth.set_auth(data)
        return data.token

    async def register(self, username: str, display_name: str, password: str) -> AuthResponse:
        r = await self._post("/v1/auth/register", {
            "username": username, "display_name": display_name, "password": password,
        })
        data = AuthResponse(**r.json())
        self._auth.set_auth(data)
        return data

    async def me(self) -> UserProfile:
        r = await self._get("/v1/auth/me")
        return UserProfile(**r.json())

    async def logout(self) -> None:
        try:
            await self._post("/v1/auth/logout")
        finally:
            self._auth.clear()

    # ── Orchestrate ────────────────────────────────────────────────────

    async def orchestrate(self, conversation_id: int, query: str) -> str:
        """Send a query to the agent system for processing."""
        r = await self._post("/api/orchestrate", {
            "conversation_id": conversation_id, "query": query,
        })
        return r.text


# ── Namespaced API Modules ────────────────────────────────────────────

class _ConversationsAPI:
    """Conversation CRUD and search."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def list(self, limit: int = 50) -> list[Conversation]:
        return await self._client._get_json("/api/conversations", {"limit": limit}, Conversation)

    async def create(self, title: str) -> int:
        return await self._client._post_json("/api/conversations", {"title": title})

    async def update_title(self, conv_id: int, title: str) -> None:
        await self._client._post("/api/conversations/update_title", {"id": conv_id, "title": title})

    async def delete(self, conv_id: int) -> None:
        await self._client._post("/api/conversations/delete", {"id": conv_id})

    async def search(self, query: str) -> list[ConversationSearchResult]:
        return await self._client._get_json("/api/conversations/search", {"q": query}, ConversationSearchResult)


class _MessagesAPI:
    """Message and attachment management."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def list(self, conversation_id: int) -> list[Message]:
        return await self._client._get_json("/api/messages", {"conversation_id": conversation_id}, Message)

    async def send(self, conversation_id: int, role: str, content: str, hide_evidence: bool = False) -> int:
        return await self._client._post_json("/api/messages", {
            "conversation_id": conversation_id, "role": role, "content": content, "hide_evidence": hide_evidence,
        })

    async def delete(self, message_id: int) -> None:
        await self._client._post("/api/messages/delete", {"id": message_id})

    async def list_attachments(self, message_id: int) -> list[Attachment]:
        return await self._client._get_json("/api/attachments", {"message_id": message_id}, Attachment)

    async def add_attachment(self, message_id: int, filename: str, file_path: str, content_type: str, size: int) -> int:
        return await self._client._post_json("/api/attachments", {
            "message_id": message_id, "filename": filename, "file_path": file_path,
            "content_type": content_type, "size": size,
        })


class _AgentsAPI:
    """Agent discovery and management."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def list(self) -> list[AgentInfo]:
        return await self._client._get_json("/api/agents", model=AgentInfo)

    async def list_all(self) -> list[AgentInfo]:
        return await self._client._get_json("/api/agents/all", model=AgentInfo)

    async def create(self, name: str, description: str, domain: str) -> int:
        return await self._client._post_json("/api/agents", {"name": name, "description": description, "domain": domain})

    async def update(self, agent: AgentInfo) -> None:
        await self._client._post("/api/agents/update", agent.model_dump())

    async def get_by_domain(self, domain: str) -> Optional[AgentInfo]:
        data = await self._client._get_json("/api/agents/by_domain", {"domain": domain})
        return AgentInfo(**data) if data else None


class _ToolsAPI:
    """Tool registration and agent linking."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def list(self) -> list[ToolInfo]:
        return await self._client._get_json("/api/tools", model=ToolInfo)

    async def create(self, name: str, description: str, language: str) -> int:
        return await self._client._post_json("/api/tools", {"name": name, "description": description, "language": language})

    async def update(self, tool: ToolInfo) -> None:
        await self._client._post("/api/tools/update", tool.model_dump())

    async def link_to_agent(self, agent_id: int, tool_id: int) -> None:
        await self._client._post("/api/tools/link", {"agent_id": agent_id, "tool_id": tool_id})

    async def unlink_from_agent(self, agent_id: int, tool_id: int) -> None:
        await self._client._post("/api/tools/unlink", {"agent_id": agent_id, "tool_id": tool_id})


class _SupervisorsAPI:
    """Supervisor management."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def list(self) -> list[Supervisor]:
        return await self._client._get_json("/api/supervisors", model=Supervisor)

    async def create(self, name: str, description: str, domain: str) -> int:
        return await self._client._post_json("/api/supervisors", {"name": name, "description": description, "domain": domain})

    async def update(self, supervisor: Supervisor) -> None:
        await self._client._post("/api/supervisors/update", supervisor.model_dump())

    async def delete(self, supervisor_id: int) -> None:
        await self._client._post("/api/supervisors/delete", {"id": supervisor_id})

    async def get_tools(self, supervisor_id: int) -> list[ToolInfo]:
        return await self._client._get_json(f"/api/supervisors/{supervisor_id}/tools", model=ToolInfo)

    async def get_agents(self, supervisor_id: int) -> list[AgentInfo]:
        return await self._client._get_json(f"/api/supervisors/{supervisor_id}/agents", model=AgentInfo)

    async def link_agent(self, supervisor_id: int, agent_id: int) -> None:
        await self._client._post("/api/supervisors/link", {"supervisor_id": supervisor_id, "agent_id": agent_id})

    async def unlink_agent(self, supervisor_id: int, agent_id: int) -> None:
        await self._client._post("/api/supervisors/unlink", {"supervisor_id": supervisor_id, "agent_id": agent_id})


class _WorkflowsAPI:
    """Workflow definitions and execution."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def list(self) -> list[Workflow]:
        return await self._client._get_json("/api/workflows", model=Workflow)

    async def create(self, name: str) -> int:
        return await self._client._post_json("/api/workflows", {"name": name})

    async def update(self, workflow: Workflow) -> None:
        await self._client._post("/api/workflows/update", workflow.model_dump())

    async def delete(self, workflow_id: int) -> None:
        await self._client._post("/api/workflows/delete", {"id": workflow_id})

    async def get_steps(self, workflow_id: int) -> list[WorkflowStep]:
        return await self._client._get_json(f"/api/workflows/{workflow_id}/steps", model=WorkflowStep)

    async def add_step(self, workflow_id: int) -> int:
        return await self._client._post_json(f"/api/workflows/{workflow_id}/steps", {})

    async def update_step(self, step: WorkflowStep) -> None:
        await self._client._post("/api/workflows/steps/update", step.model_dump())

    async def delete_step(self, step_id: int) -> None:
        await self._client._post("/api/workflows/steps/delete", {"id": step_id})

    async def get_runs(self, workflow_id: int) -> list[WorkflowRun]:
        return await self._client._get_json(f"/api/workflows/{workflow_id}/runs", model=WorkflowRun)

    async def get_step_runs(self, run_id: int) -> list[WorkflowStepRun]:
        return await self._client._get_json(f"/api/workflows/runs/{run_id}/steps", model=WorkflowStepRun)

    async def run(self, workflow_id: int) -> int:
        return await self._client._post_json(f"/api/workflows/{workflow_id}/run", {})

    async def publish(self, workflow_id: int) -> None:
        await self._client._post(f"/v1/workflows/{workflow_id}/publish", {})

    async def get_versions(self, workflow_id: int) -> list[dict]:
        return await self._client._get_json(f"/v1/workflows/{workflow_id}/versions")

    async def restore_version(self, workflow_id: int, version: int) -> None:
        await self._client._post(f"/v1/workflows/{workflow_id}/versions/{version}/restore", {})


class _WorkItemsAPI:
    """Work item (task) management."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def list(self, conversation_id: int) -> list[WorkItem]:
        return await self._client._get_json("/api/work_items", {"conversation_id": conversation_id}, WorkItem)

    async def create(self, **kwargs: Any) -> int:
        return await self._client._post_json("/api/work_items", kwargs)

    async def update_status(self, item_id: int, status: str) -> None:
        await self._client._post("/api/work_items/update_status", {"id": item_id, "status": status})

    async def update_runtime(self, item_id: int, status: str, metadata_json: Optional[str] = None) -> None:
        await self._client._post("/api/work_items/update_runtime", {
            "id": item_id, "status": status, "metadata_json": metadata_json,
        })

    async def log_event(self, **kwargs: Any) -> int:
        return await self._client._post_json("/api/work_items/event", kwargs)

    async def get_events(self, work_item_id: int, limit: int = 50) -> list[WorkItemEvent]:
        return await self._client._get_json(f"/api/work_items/{work_item_id}/events", {"limit": limit}, WorkItemEvent)


class _ArtifactsAPI:
    """Artifact storage and retrieval."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def list(self, conversation_id: int) -> list[Artifact]:
        return await self._client._get_json("/api/artifacts", {"conversation_id": conversation_id}, Artifact)

    async def create(self, **kwargs: Any) -> int:
        return await self._client._post_json("/api/artifacts", kwargs)

    async def get_by_ids(self, ids: list[int]) -> list[Artifact]:
        return await self._client._post_json("/api/artifacts/by_ids", ids, Artifact)

    async def get_by_run(self, execution_run_id: int) -> list[Artifact]:
        return await self._client._get_json("/api/artifacts/by_run", {"execution_run_id": execution_run_id}, Artifact)


class _PlannersAPI:
    """Planner configuration."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def list(self) -> list[Planner]:
        return await self._client._get_json("/api/planners", model=Planner)

    async def create(self, **kwargs: Any) -> int:
        return await self._client._post_json("/api/planners", kwargs)


class _SupervisionAPI:
    """Computer supervision and kernel approvals."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def get_state(self) -> ComputerSupervisionState:
        return await self._client._get_json("/api/supervision/state", model=ComputerSupervisionState)

    async def approve_action(self) -> None:
        await self._client._post("/api/supervision/approve", {})

    async def reject_action(self) -> None:
        await self._client._post("/api/supervision/reject", {})

    async def activate(self, enabled: bool, allowlist: str = "") -> None:
        await self._client._post("/api/supervision/activate", {"enabled": enabled, "allowlist": allowlist})

    async def clear_pending(self) -> None:
        await self._client._post("/api/supervision/clear_pending", {})

    async def list_pending_approvals(self) -> list[dict]:
        return await self._client._get_json("/api/kernel/approvals")

    async def kernel_approve(self, approval_id: str) -> dict:
        return await self._client._post_json("/api/kernel/approve", {"approval_id": approval_id, "resolver": "api"})

    async def kernel_reject(self, approval_id: str) -> dict:
        return await self._client._post_json("/api/kernel/reject", {"approval_id": approval_id, "resolver": "api"})

    async def kernel_emergency_stop(self, run_id: Optional[int] = None) -> dict:
        return await self._client._post_json("/api/kernel/emergency_stop", {"run_id": run_id})


class _VersionsAPI:
    """Entity version publishing and rollback."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def publish(self, entity_type: str, entity_id: int, tag: str, content: str, description: str = "") -> None:
        await self._client._post("/api/version/publish", {
            "entity_type": entity_type, "entity_id": entity_id, "tag": tag,
            "content": content, "description": description,
        })

    async def publish_agent(self, agent_id: int, tag: str, description: str = "") -> None:
        await self._client._post("/api/version/publish_agent", {"agent_id": agent_id, "tag": tag, "description": description})

    async def publish_tool(self, tool_id: int, tag: str, description: str = "") -> None:
        await self._client._post("/api/version/publish_tool", {"tool_id": tool_id, "tag": tag, "description": description})

    async def rollback(self, entity_type: str, entity_id: int, tag: str) -> None:
        await self._client._post("/api/version/rollback", {"entity_type": entity_type, "entity_id": entity_id, "tag": tag})

    async def get_content(self, entity_type: str, entity_id: int, tag: str) -> str:
        r = await self._client._get("/api/version/content", {"entity_type": entity_type, "entity_id": entity_id, "tag": tag})
        return r.text

    async def get_history(self, entity_type: str, entity_id: int) -> list[EntityVersion]:
        return await self._client._get_json("/api/version/history", {"entity_type": entity_type, "entity_id": entity_id}, EntityVersion)


class _BrowserAPI:
    """Browser session management and automation."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def create_session(self) -> dict:
        return await self._client._post_json("/v1/browser/sessions", {})

    async def get_session(self, session_id: int) -> BrowserSessionInfo:
        return await self._client._get_json(f"/v1/browser/sessions/{session_id}", model=BrowserSessionInfo)

    async def close_session(self, session_id: int) -> None:
        await self._client._delete(f"/v1/browser/sessions/{session_id}")

    async def navigate(self, session_id: int, url: str) -> dict:
        return await self._client._post_json(f"/v1/browser/sessions/{session_id}/navigate", {"url": url})

    async def screenshot(self, session_id: int) -> dict:
        return await self._client._get_json(f"/v1/browser/sessions/{session_id}/screenshot")

    async def set_policy(self, session_id: int, policy: dict) -> None:
        await self._client._post(f"/v1/browser/sessions/{session_id}/policy", policy)

    async def get_audit_log(self, session_id: int) -> list[dict]:
        return await self._client._get_json(f"/v1/browser/sessions/{session_id}/audit")

    async def memory_report(self) -> dict:
        return await self._client._get_json("/v1/browser/memory")

    async def start_recording(self, session_id: int, quality: int = 75) -> dict:
        return await self._client._post_json(f"/v1/browser/sessions/{session_id}/recording/start", {"quality": quality})

    async def stop_recording(self, session_id: int) -> dict:
        return await self._client._post_json(f"/v1/browser/sessions/{session_id}/recording/stop", {})

    async def get_recording(self, session_id: int) -> dict:
        return await self._client._get_json(f"/v1/browser/sessions/{session_id}/recording")

    async def get_recording_events(self, session_id: int) -> list[dict]:
        return await self._client._get_json(f"/v1/browser/sessions/{session_id}/recording/events")

    async def resume_hitl(self, session_id: int) -> None:
        await self._client._post(f"/v1/browser/sessions/{session_id}/resume", {})


class _CredentialsAPI:
    """Credential storage."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def list(self) -> list[CredentialSummary]:
        return await self._client._get_json("/v1/credentials", model=CredentialSummary)

    async def create(self, name: str, value: str, cred_type: str = "api_key") -> CredentialSummary:
        return await self._client._post_json("/v1/credentials", {"name": name, "value": value, "cred_type": cred_type}, CredentialSummary)

    async def update(self, credential_id: int, name: Optional[str] = None, value: Optional[str] = None) -> None:
        body: dict[str, Any] = {}
        if name is not None:
            body["name"] = name
        if value is not None:
            body["value"] = value
        await self._client._patch(f"/v1/credentials/{credential_id}", body)

    async def delete(self, credential_id: int) -> None:
        await self._client._delete(f"/v1/credentials/{credential_id}")


class _TemplatesAPI:
    """Workflow template management."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def list(self) -> list[WorkflowTemplate]:
        return await self._client._get_json("/v1/workflow-templates", model=WorkflowTemplate)

    async def create(self, name: str, graph_json: str, description: Optional[str] = None, category: str = "custom") -> WorkflowTemplate:
        return await self._client._post_json("/v1/workflow-templates", {
            "name": name, "graph_json": graph_json, "description": description, "category": category,
        }, WorkflowTemplate)

    async def install(self, template_id: int) -> int:
        return await self._client._post_json(f"/v1/workflow-templates/{template_id}/install", {})


class _SettingsAPI:
    """Key-value settings."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def get(self, key: str) -> Optional[str]:
        r = await self._client._get("/api/settings", {"key": key})
        return r.json()

    async def set(self, key: str, value: str) -> None:
        await self._client._post("/api/settings", {"key": key, "value": value})

    async def get_all(self) -> dict[str, str]:
        return await self._client._get_json("/api/settings/all")


class _SystemAPI:
    """Health, metrics, and system information."""

    def __init__(self, client: OverseerClient) -> None:
        self._client = client

    async def health(self) -> ServiceHealth:
        return await self._client._get_json("/healthz", model=ServiceHealth)

    async def runtime_health(self) -> RuntimeHealth:
        return await self._client._get_json("/api/health", model=RuntimeHealth)

    async def load(self) -> SystemLoad:
        return await self._client._get_json("/api/system/load", model=SystemLoad)

    async def models(self) -> list[str]:
        return await self._client._get_json("/api/models/available")

    async def registry_status(self) -> RuntimeRegistryStatus:
        return await self._client._get_json("/api/registry/status", model=RuntimeRegistryStatus)

    async def refresh_registry(self) -> None:
        await self._client._post("/api/registry/refresh", {})
