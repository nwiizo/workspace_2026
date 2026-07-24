import { FC, RefObject, useEffect, useState } from "react";
import { Toggle } from "~/components/primitives/form/toggle";
import { ConfigFrame } from "~/components/primitives/frame/config-frame";
import { Text } from "~/components/primitives/text/text";
import {
  Message,
  MessageTypes,
  sendSimulatorConfig,
} from "~/utils/post-message";

type SimulatorConfigType = {
  ghostChairEnabled: boolean;
};

export const SimulatorGhostChairToggle: FC<{
  simulatorRef: RefObject<HTMLIFrameElement>;
}> = ({ simulatorRef }) => {
  const [ready, setReady] = useState<boolean>(false);

  const [config, setConfig] = useState<SimulatorConfigType>({
    ghostChairEnabled: true,
  });

  useEffect(() => {
    if (!ready) return;
    if (simulatorRef.current?.contentWindow) {
      sendSimulatorConfig(simulatorRef.current.contentWindow, config);
    }
  }, [config, ready, simulatorRef]);

  useEffect(() => {
    const onMessage = ({ data }: MessageEvent<Message["ClientReady"]>) => {
      const isSameOrigin = origin == location.origin;
      if (isSameOrigin && data.type === MessageTypes.ClientReady) {
        setReady(Boolean(data?.payload?.ready));
      }
    };
    window.addEventListener("message", onMessage);
    return () => {
      window.removeEventListener("message", onMessage);
    };
  }, []);

  return (
    <ConfigFrame>
      <div className="flex justify-between items-center">
        <Text size="sm" className="text-neutral-500" bold>
          疑似チェアを表示する
        </Text>
        <Toggle
          id="ghost-chair"
          checked={config.ghostChairEnabled}
          onUpdate={(v) => {
            setConfig((c) => ({ ...c, ghostChairEnabled: v }));
          }}
        />
      </div>
    </ConfigFrame>
  );
};
