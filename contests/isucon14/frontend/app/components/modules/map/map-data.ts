import { Coordinate } from "~/types";

type Image = {
  src: `/${string}`;
  width: number;
  height: number;
};
type CityObject = { image: Image; coordinate: Coordinate };

type Town = {
  name: string;
  image: Image;
  centerCoordinate: Coordinate;
  color: `#${string}`;
};

const CityObjectImages = [
  { src: "/images/buildings1.svg", width: 170, height: 100 },
  { src: "/images/buildings2.svg", width: 112, height: 64 },
  { src: "/images/buildings3.svg", width: 112, height: 64 },
  { src: "/images/buildings4.svg", width: 101, height: 38 },
  { src: "/images/buildings5.svg", width: 79, height: 32 },
  { src: "/images/buildings6.svg", width: 125, height: 60 },
  { src: "/images/house1.svg", width: 81, height: 46 },
  { src: "/images/house2.svg", width: 71, height: 53 },
] as const;

// prettier-ignore
const CityObjectCoordinates = [
  [-307,-327],[310,-237],[165,-347],[178,-126],[71,-194],
  [-148,352],[-129,90],[248,149],[-261,-295],[-274,397],
  [-380,-215],[-132,124],[-235,-63],[-316,104],[-420,-120],
  [386,117],[-115,-65],[-55,-375],[194,-315],[108,-180],
  [399,-330],[-179,175],[-93,-39],[128,-217],[245,181],
  [351,66],[140,92],[-117,-97],[-154,-202],[-63,-188],
  [191,241],[123,211],[-341,173],[-163,229],[151,63],
  [-104,-137],[-200,-301],[-345,-88],[-216,-107],[-387,-275],
  [-242,-389],[-323,-36],[-276,-359],[-346,-239],[176,394],
  [110,51],[-358,78],[-329,-376],[397,-248],[-288,363],
  [243,-331],[241,210],[91,-395],[57,391],[-73,-220],
  [385,375],[-359,150],[-166,-363],[212,146],[366,-204],
  [190,207],[-80,-156],[-167,-428],[87,-96],[379,-261],
  [-347,222],[-134,205],[-243,226],[-289,-113],[-219,146],
  [364,-399],[227,-203],[53,-161],[163,-162],[209,108],
  [-277,-249],[-430,-334],[-81,-279],[-168,-51],[-250,145]
];

export const CityObjects = CityObjectCoordinates.map(
  ([latitude, longitude], i) => ({
    image: CityObjectImages[i % CityObjectImages.length],
    coordinate: { latitude, longitude },
  }),
) satisfies CityObject[];

export const TownList = [
  {
    name: "チェアタウン",
    centerCoordinate: { latitude: 0, longitude: 0 },
    image: { src: "/images/town.svg", width: 500, height: 500 },
    color: "#FF3600",
  },
  {
    name: "コシカケシティ",
    centerCoordinate: { latitude: 300, longitude: 300 },
    image: { src: "/images/town.svg", width: 500, height: 500 },
    color: "#0089A2",
  },
] satisfies Town[];
