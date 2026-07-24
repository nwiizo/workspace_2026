export const baseUrl = "https://isuride.xiv.isucon.net";
export const randomString = Math.floor(Math.random() * 36 ** 6).toString(36);

/** @type {Array<{ path: string, selector: string }>} */
export const pages = [
	// "/client/register", // 別処理
	// "/client/register-payment", // 別処理
	// "/owner/register", // 別処理
	// "/owner/login", // 別処理
	{ path: "/client", selector: "nav" },
	{ path: "/client/history", selector: "nav" },
	{ path: "/owner", selector: "table" },
	{ path: "/owner/sales", selector: "table" },
];

// format: "teamId\tip"
const raw_teams = `
14	57.182.82.181
`;

/** @type {Array<{teamId: number, ip: string}>} */
export const teams = raw_teams
	.trim()
	.split("\n")
	.map(line => {
		const [teamId, ip] = line.split("\t");
		return { teamId: Number(teamId), ip };
	});
