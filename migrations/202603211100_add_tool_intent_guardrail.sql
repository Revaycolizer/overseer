-- Migration: 202603211100_add_tool_intent_guardrail.sql
-- Add required_intent column to tools table for dynamic hallucination prevention.

ALTER TABLE tools ADD COLUMN required_intent TEXT;

-- Seed charting tool with its existing keywords
UPDATE tools SET required_intent = 'chart,graph,plot,visualize,visualization,draw,sketch,diagram,pie,bar,mapped' WHERE name = 'charting';
