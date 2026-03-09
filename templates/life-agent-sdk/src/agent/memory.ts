export class AgentMemory {
  private readonly memory = new Map<string, string>();

  set(key: string, value: string): void {
    this.memory.set(key, value);
  }

  get(key: string): string | undefined {
    return this.memory.get(key);
  }
}
