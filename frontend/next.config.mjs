/** @type {import('next').NextConfig} */
const nextConfig = {
// TODO: something is broken when two requests happen in quick succession
// (like the annoying strict mode checks).
  reactStrictMode: false,
  webpack: (config, { isServer }) => {
    if (!isServer) {
      config.resolve.fallback = {
        ...config.resolve.fallback,
        fs: false,
      };
    }
    return config;
  },
}

export default nextConfig
