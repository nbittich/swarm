FROM node:20-alpine AS build
WORKDIR /app
COPY package.json package.json
RUN npm install
COPY . .
RUN npm run build
FROM nginx:1.27-alpine
COPY --from=build /app/dist /usr/share/nginx/html
COPY env.sh /docker-entrypoint.d/env.sh
COPY nginx.conf /etc/nginx/nginx.conf
RUN chmod +x /docker-entrypoint.d/env.sh
EXPOSE 80
