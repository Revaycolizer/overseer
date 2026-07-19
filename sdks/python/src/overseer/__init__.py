"""Overseer Python SDK — Python client for the Overseer AI orchestration platform.

Install:
    pip install overseer-pysdk

Usage:
    async with OverseerClient("http://localhost:8080", token="...") as client:
        conversations = await client.conversations.list()
        agents = await client.agents.list()
"""

from .client import OverseerClient
from .models import (
    AgentInfo, Artifact, Attachment, AuthResponse, BrowserSessionInfo,
    ComputerSupervisionState, Conversation, ConversationSearchResult,
    CredentialSummary, EntityVersion, Message, Planner, RuntimeHealth,
    RuntimeRegistryStatus, Supervisor, SystemLoad, ToolInfo, UserProfile,
    WorkItem, WorkItemEvent, Workflow, WorkflowRun, WorkflowStep,
    WorkflowStepRun, WorkflowTemplate,
)

__all__ = [
    "OverseerClient",
    "AgentInfo", "Artifact", "Attachment", "AuthResponse", "BrowserSessionInfo",
    "ComputerSupervisionState", "Conversation", "ConversationSearchResult",
    "CredentialSummary", "EntityVersion", "Message", "Planner", "RuntimeHealth",
    "RuntimeRegistryStatus", "Supervisor", "SystemLoad", "ToolInfo", "UserProfile",
    "WorkItem", "WorkItemEvent", "Workflow", "WorkflowRun", "WorkflowStep",
    "WorkflowStepRun", "WorkflowTemplate",
]
__version__ = "0.1.1"
