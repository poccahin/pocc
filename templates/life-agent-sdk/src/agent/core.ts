export class AgentCore {
  constructor(public readonly name: string) {}

  run(task: string): string {
    return `[${this.name}] executing task: ${task}`;
  }
}
