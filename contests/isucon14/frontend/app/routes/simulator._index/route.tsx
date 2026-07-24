import type { MetaFunction } from "@remix-run/react";
import { useEffect, useRef } from "react";
import { fetchChairPostActivity } from "~/api/api-components";
import { useEmulator } from "~/components/hooks/use-emulator";
import { SimulatorChairActiveToggle } from "~/components/modules/simulator-configs/simulator-chair-active-toggle";
import { SimulatorChairConfig } from "~/components/modules/simulator-configs/simulator-chair-config";
import { SimulatorGhostChairToggle } from "~/components/modules/simulator-configs/simulator-ghost-chair-toggle";
import { SmartPhone } from "~/components/primitives/smartphone/smartphone";

export const meta: MetaFunction = () => {
  return [
    { title: "Simulator | ISURIDE" },
    { name: "description", content: "isucon14" },
  ];
};

export default function Index() {
  const ref = useRef<HTMLIFrameElement>(null);

  useEmulator();

  useEffect(() => {
    try {
      void fetchChairPostActivity({ body: { is_active: true } });
    } catch (error) {
      console.error(error);
    }
  }, []);

  return (
    <main className="h-screen flex justify-center items-center space-x-8 lg:space-x-16">
      <SmartPhone>
        <iframe
          title="ISURIDE Client App"
          src="/client"
          className="w-full h-full"
          ref={ref}
        />
      </SmartPhone>
      <div className="space-y-4 min-w-[320px] lg:w-[400px]">
        <h1 className="text-lg font-semibold mb-4">Chair Simulator</h1>
        <SimulatorChairConfig />
        <SimulatorChairActiveToggle />
        <SimulatorGhostChairToggle simulatorRef={ref} />
      </div>
    </main>
  );
}
