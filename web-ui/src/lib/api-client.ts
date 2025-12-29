import { client } from "@/generated/client.gen";

// Configure the API client
// In development, requests go through Vite proxy
// In production on Raspberry Pi, use relative URLs
client.setConfig({
  baseUrl: import.meta.env.DEV ? "/api" : "/api",
});

export { client };
