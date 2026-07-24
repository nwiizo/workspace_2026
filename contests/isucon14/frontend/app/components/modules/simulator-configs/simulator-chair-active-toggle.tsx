import { FC, useCallback, useState } from "react";
import { fetchChairPostActivity } from "~/api/api-components";
import { Toggle } from "~/components/primitives/form/toggle";
import { ConfigFrame } from "~/components/primitives/frame/config-frame";
import { Text } from "~/components/primitives/text/text";
import { useSimulatorContext } from "~/contexts/simulator-context";

export const SimulatorChairActiveToggle: FC = () => {
  const { chair, isAnotherSimulatorBeingUsed } = useSimulatorContext();
  const [activate, setActivate] = useState<boolean>(true);

  const toggleActivate = useCallback(
    (activity: boolean) => {
      try {
        void fetchChairPostActivity({ body: { is_active: activity } });
        setActivate(activity);
      } catch (error) {
        console.error(error);
      }
    },
    [setActivate],
  );

  if (!chair) {
    return null;
  }

  return (
    <ConfigFrame aria-disabled={isAnotherSimulatorBeingUsed}>
      <div className="flex justify-between items-center">
        <Text size="sm" className="text-neutral-500" bold>
          配車を受け付ける
        </Text>
        <Toggle
          checked={activate}
          onUpdate={(v) => toggleActivate(v)}
          id="chair-activity"
        />
      </div>
      {isAnotherSimulatorBeingUsed && (
        <div
          role="presentation"
          className="absolute top-0 left-0 w-full h-full bg-neutral-500 bg-opacity-60 flex items-center justify-center cursor-not-allowed"
        />
      )}
    </ConfigFrame>
  );
};
