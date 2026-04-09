import { invoke } from "@tauri-apps/api/core";
import type {
  GraphResponse,
  NodeDto,
} from "$lib/types";

export async function listNodes(limit: number, sessionId?: string): Promise<{ nodes: NodeDto[] }> {
  return invoke<{ nodes: NodeDto[] }>("list_nodes", { limit, sessionId });
}

export async function getGraph(limit: number, sessionId?: string): Promise<GraphResponse> {
  return invoke<GraphResponse>("get_graph", { limit, sessionId });
}
