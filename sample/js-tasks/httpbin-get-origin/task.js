/**
 * HTTPBin Origin Task
 * Calls https://httpbin.org/get and returns the origin IP address
 */

async function main(input) {
  const response = await fetch('https://httpbin.org/get');
  const data = await response.json();
  return { origin: data.origin };
}