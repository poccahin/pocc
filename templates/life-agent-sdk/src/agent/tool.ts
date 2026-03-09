export type ToolHandler = (input: string) => Promise<string>;

export class AgentTool {
  constructor(
    public readonly name: string,
    private readonly handler: ToolHandler,
  ) {}

  async call(input: string): Promise<string> {
    return this.handler(input);
  }
}
