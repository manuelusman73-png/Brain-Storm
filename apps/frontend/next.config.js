const createNextIntlPlugin = require('next-intl/plugin');

const withNextIntl = createNextIntlPlugin('./src/i18n/request.ts');

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  output: 'standalone',
  env: {
    NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL,
    NEXT_PUBLIC_STELLAR_NETWORK: process.env.NEXT_PUBLIC_STELLAR_NETWORK,
    NEXT_PUBLIC_SENTRY_DSN: process.env.NEXT_PUBLIC_SENTRY_DSN,
    NEXT_PUBLIC_GIT_COMMIT_SHA: process.env.NEXT_PUBLIC_GIT_COMMIT_SHA,
  },
  images: {
    remotePatterns: [
      // Google / Gravatar avatars (OAuth)
      { protocol: 'https', hostname: 'lh3.googleusercontent.com' },
      { protocol: 'https', hostname: 'www.gravatar.com' },
      // Generic HTTPS avatars supplied by users
      { protocol: 'https', hostname: '**' },
    ],
  },
};

module.exports = withNextIntl(nextConfig);
