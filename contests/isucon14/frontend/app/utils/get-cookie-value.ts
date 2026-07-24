export const getCookieValue = (cookieString: string, cookieName: string) => {
  const regex = new RegExp(`(?:^|; )${cookieName}=([^;]*)`);
  const match = cookieString.match(regex);
  return match ? match[1] : undefined;
};
