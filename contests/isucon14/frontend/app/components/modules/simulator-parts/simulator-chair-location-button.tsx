import { FC, useCallback, useRef, useState } from "react";
import { Button } from "~/components/primitives/button/button";
import { Modal } from "~/components/primitives/modal/modal";
import type { Coordinate } from "~/types";
import { LocationButton } from "../location-button/location-button";
import { Map } from "../map/map";

export const SimulatorChairLocationButton: FC<{
  coordinate?: Coordinate;
  setCoordinate?: (coordinate: Coordinate) => void;
}> = ({ coordinate, setCoordinate }) => {
  const [initialMapLocation, setInitialMapLocation] = useState<Coordinate>();
  const [mapLocation, setMapLocation] = useState<Coordinate>();
  const [visibleModal, setVisibleModal] = useState<boolean>(false);
  const modalRef = useRef<HTMLElement & { close: () => void }>(null);

  const handleOpenModal = useCallback(() => {
    setInitialMapLocation(coordinate);
    setVisibleModal(true);
  }, [coordinate]);

  const handleCloseModal = useCallback(() => {
    if (mapLocation) {
      setCoordinate?.(mapLocation);
    }
    modalRef.current?.close();
    setVisibleModal(false);
  }, [mapLocation, setCoordinate]);

  return (
    <>
      <LocationButton
        className="w-full text-right"
        location={coordinate}
        label="椅子位置"
        placeholder="現在位置を設定"
        onClick={handleOpenModal}
      />
      {visibleModal && (
        <div className="fixed inset-0 z-10">
          <Modal
            ref={modalRef}
            center
            onClose={handleCloseModal}
            className="absolute w-full max-w-[800px] max-h-none h-[700px]"
          >
            <div className="w-full h-full flex flex-col items-center">
              <Map
                className="flex-1"
                initialCoordinate={initialMapLocation}
                from={initialMapLocation}
                onMove={(c) => setMapLocation(c)}
                selectable
              />
              <Button
                className="w-full mt-6"
                onClick={handleCloseModal}
                variant="primary"
              >
                この位置で確定する
              </Button>
            </div>
          </Modal>
        </div>
      )}
    </>
  );
};
