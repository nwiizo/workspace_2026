import { FC } from "react";
import { ChairWaitingIndicator } from "~/components/modules/chair-waiting-indicator/chair-waiting-indicator";
import { LocationButton } from "~/components/modules/location-button/location-button";
import { ModalHeader } from "~/components/modules/modal-header/moda-header";
import { Price } from "~/components/modules/price/price";
import { Text } from "~/components/primitives/text/text";
import { useClientContext } from "~/contexts/client-context";
import { Coordinate } from "~/types";

export const Matching: FC<{
  optimistic: {
    pickup?: Coordinate;
    destLocation?: Coordinate;
    fare?: number;
  };
}> = ({ optimistic }) => {
  const { data } = useClientContext();
  const fare = optimistic.fare ?? data?.fare;
  const destLocation = optimistic.destLocation ?? data?.destination_coordinate;
  const pickup = optimistic.pickup ?? data?.pickup_coordinate;
  return (
    <div className="w-full h-full px-8 flex flex-col items-center justify-center">
      <ModalHeader title="マッチング中" subTitle="椅子をさがしています">
        <ChairWaitingIndicator size={120} />
      </ModalHeader>
      <LocationButton
        label="現在地"
        location={pickup}
        className="w-80"
        disabled
      />
      <Text size="xl">↓</Text>
      <LocationButton
        label="目的地"
        location={destLocation}
        className="w-80"
        disabled
      />
      {fare && <Price value={fare} pre="運賃" className="mt-6" />}
    </div>
  );
};
