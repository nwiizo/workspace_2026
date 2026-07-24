import initalOwnerJson from "./initial-owner-data.json" with { type: "json" };

/**
 * Sleep function to pause execution for the specified time.
 * @param {number} ms - The number of milliseconds to wait.
 * @returns {Promise<void>} A Promise that resolves after the specified duration.
 */
export function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

const main = async () => {
  const targetOwner = initalOwnerJson.owners[0];
  const targetChair = targetOwner.chairs[0];
  let currentCoordinate = {
    latitude: 300,
    longitude: 300,
  };

  // activity on
  await fetch("http://localhost:8080/api/chair/activity", {
    method: "POST",
    body: JSON.stringify({
      is_active: true,
    }),
    headers: {
      Cookie: `chair_session=${targetChair.token}`,
    },
  });

  // coordinate on
  await fetch("http://localhost:8080/api/chair/coordinate", {
    method: "POST",
    body: JSON.stringify(currentCoordinate),
    headers: {
      Cookie: `chair_session=${targetChair.token}`,
    },
  });

  while (true) {
    await sleep(1000);
    const fetched = await fetch(
      "http://localhost:8080/api/chair/notification",
      {
        method: "GET",
        headers: {
          Cookie: `chair_session=${targetChair.token}`,
        },
      },
    );
    /**
     * @type {{status: string, ride_id: string, pickup_coordinate: {latitude: number, longitude: number}, destination_coordinate: {latitude: number, longitude: number}}}
     */
    let json;
    try {
      json = await fetched.json();
    } catch (e) {
      continue;
    }
    console.log(json);
    const { status, ride_id, pickup_coordinate } = json;

    if (status === undefined) {
    }
    if (status === "MATCHING") {
      await sleep(10000);
      await fetch(`http://localhost:8080/api/chair/rides/${ride_id}/status`, {
        method: "POST",
        headers: {
          Cookie: `chair_session=${targetChair.token}`,
        },
        body: JSON.stringify({ status: "ENROUTE" }),
      });
      while (true) {
        await sleep(5000);
        await fetch("http://localhost:8080/api/chair/coordinate", {
          method: "POST",
          body: JSON.stringify({
            latitude:
              currentCoordinate.latitude +
              (pickup_coordinate.latitude - currentCoordinate.latitude) / 2,
            longitude:
              currentCoordinate.longitude +
              (pickup_coordinate.longitude - currentCoordinate.longitude) / 2,
          }),
          headers: {
            Cookie: `chair_session=${targetChair.token}`,
          },
        });
        await sleep(5000);
        await fetch("http://localhost:8080/api/chair/coordinate", {
          method: "POST",
          body: JSON.stringify(pickup_coordinate),
          headers: {
            Cookie: `chair_session=${targetChair.token}`,
          },
        });
        currentCoordinate = pickup_coordinate;
        break;
      }
      break;
    }
  }

  console.log("enroute");
  while (true) {
    await sleep(1000);
    const fetched = await fetch(
      "http://localhost:8080/api/chair/notification",
      {
        method: "GET",
        headers: {
          Cookie: `chair_session=${targetChair.token}`,
        },
      },
    );
    /**
     * @type {{status: string, ride_id: string, pickup_coordinate: {latitude: number, longitude: number}, destination_coordinate: {latitude: number, longitude: number}}}
     */
    let json;
    try {
      json = await fetched.json();
    } catch (e) {
      continue;
    }
    console.log(json);
    const { status, ride_id, destination_coordinate } = json;

    if (status === undefined) {
    }
    if (status === "PICKUP") {
      await sleep(10000);
      await fetch(`http://localhost:8080/api/chair/rides/${ride_id}/status`, {
        method: "POST",
        headers: {
          Cookie: `chair_session=${targetChair.token}`,
        },
        body: JSON.stringify({ status: "CARRYING" }),
      });
    }
    while (true) {
      await sleep(5000);
      await fetch("http://localhost:8080/api/chair/coordinate", {
        method: "POST",
        body: JSON.stringify({
          latitude:
            currentCoordinate.latitude +
            (destination_coordinate.latitude - currentCoordinate.latitude) / 2,
          longitude:
            currentCoordinate.longitude +
            (destination_coordinate.longitude - currentCoordinate.longitude) /
              2,
        }),
        headers: {
          Cookie: `chair_session=${targetChair.token}`,
        },
      });
      await sleep(5000);
      await fetch("http://localhost:8080/api/chair/coordinate", {
        method: "POST",
        body: JSON.stringify(destination_coordinate),
        headers: {
          Cookie: `chair_session=${targetChair.token}`,
        },
      });
      break;
    }
    break;
  }
};

main();
