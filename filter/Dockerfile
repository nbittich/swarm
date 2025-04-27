FROM maven:3.9-eclipse-temurin-24 AS build

WORKDIR /app

COPY pom.xml .

RUN mvn dependency:go-offline -B

COPY src ./src

RUN mvn package -DskipTests

FROM eclipse-temurin:24-jre-alpine

RUN apk add --no-cache tzdata
RUN ln -s /usr/share/zoneinfo/Europe/Brussels /etc/localtime

WORKDIR /app

COPY --from=build /app/target/filter.jar app.jar



ENTRYPOINT ["java","-XX:+UseZGC","-jar","/app/app.jar"]
