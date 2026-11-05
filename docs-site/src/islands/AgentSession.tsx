import { AgentDemo } from "@steamfield/ui/react/agent-demo";
import type { DemoScript } from "@steamfield/ui/script";

export type AgentSessionProps = {
  request: string;
  steps: { command: string; output: string }[];
  answer: string;
  caption?: string;
};

export function AgentSession({
  request,
  steps,
  answer,
  caption,
}: AgentSessionProps) {
  const script: DemoScript = {
    steps: [
      { kind: "user", text: request },
      ...steps.map(({ command, output }) => ({
        kind: "tool-call" as const,
        name: "terminal",
        args: command,
        output: output.split("\n"),
      })),
      { kind: "agent-text", text: answer },
    ],
  };
  return (
    <figure
      data-demo=""
      data-nosnippet=""
      className="my-8 text-base leading-normal"
    >
      <AgentDemo
        script={script}
        layout="terminal"
        glyphs={{ user: "", agent: "", ok: "", fail: "" }}
      />
      {caption && (
        <figcaption className="mt-3 text-xs text-muted-foreground">
          {caption}
        </figcaption>
      )}
    </figure>
  );
}
