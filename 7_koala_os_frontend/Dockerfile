# Koala OS Frontend — React/Three Fiber Omnisphere
FROM node:22-alpine AS builder

WORKDIR /app

# Copy package manifests for layer caching
COPY package.json package-lock.json .npmrc ./

# Install dependencies (legacy-peer-deps resolves @solana/kit conflicts)
RUN npm ci --legacy-peer-deps

# Copy source and build
COPY . .
RUN npm run build

# Production stage — serve with nginx
FROM nginx:alpine

COPY --from=builder /app/dist /usr/share/nginx/html

# Optional: custom nginx config for SPA routing
RUN printf 'server {\n  listen 3000;\n  root /usr/share/nginx/html;\n  index index.html;\n  location / { try_files $uri $uri/ /index.html; }\n}\n' \
    > /etc/nginx/conf.d/default.conf

EXPOSE 3000

LABEL org.opencontainers.image.title="Koala OS Frontend" \
      org.opencontainers.image.description="Life++ L7 Omnisphere Command Center" \
      org.opencontainers.image.source="https://github.com/poccahin/pocc"
