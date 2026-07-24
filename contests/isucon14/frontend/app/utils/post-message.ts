export const MessageTypes = {
  ClientReady: "isuride.client.ready", // クライアントの画面準備完了
  ClientRideRequested: "isuride.client.running", // クライアントでISURIDEが実行中
  SimulatorConfing: "isuride.simulator.config", // シミュレーターからの設定値変更
} as const;

export type Message = {
  ClientReady: {
    type: typeof MessageTypes.ClientReady;
    payload: { ready?: boolean };
  };
  ClientRideRequested: {
    type: typeof MessageTypes.ClientRideRequested;
    payload: { rideId?: string };
  };
  SimulatorConfing: {
    type: typeof MessageTypes.SimulatorConfing;
    payload: {
      ghostChairEnabled?: boolean;
    };
  };
};

export const sendClientReady = (
  target: Window,
  payload: NonNullable<Message["ClientReady"]["payload"]>,
) => {
  target.postMessage({ type: MessageTypes.ClientReady, payload }, "*");
};

export const sendClientRideRequested = (
  target: Window,
  payload: NonNullable<Message["ClientRideRequested"]["payload"]>,
) => {
  target.postMessage({ type: MessageTypes.ClientRideRequested, payload }, "*");
};

export const sendSimulatorConfig = (
  target: Window,
  payload: NonNullable<Message["SimulatorConfing"]["payload"]>,
) => {
  target.postMessage({ type: MessageTypes.SimulatorConfing, payload }, "*");
};
