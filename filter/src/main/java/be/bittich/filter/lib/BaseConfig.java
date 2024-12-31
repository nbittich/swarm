package be.bittich.filter.lib;

import java.io.IOException;

import org.apache.jena.riot.Lang;
import org.apache.jena.shacl.Shapes;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.core.io.Resource;

import io.nats.client.Connection;
import io.nats.client.JetStreamManagement;
import io.nats.client.Nats;
import io.nats.client.Options;
import lombok.extern.slf4j.Slf4j;

@Slf4j
@Configuration
public class BaseConfig {
  @Bean
  public Connection connection(@Value("${nats.connection.url}") String connectionUrl,
      @Value("${nats.connection.username}") String username,
      @Value("${nats.connection.maxReconnects}") int maxReconnects,
      @Value("${nats.connection.password}") String password)
      throws IllegalStateException, IOException, InterruptedException {

    return Nats.connect(Options.builder()
        .server(connectionUrl)
        .maxReconnects(maxReconnects)
        .userInfo(username, password)
        .build());
  }

  @Bean
  public JetStreamManagement jetStreamManagement(@Autowired Connection conn) throws IOException {
    return conn.jetStreamManagement();
  }

  @Bean
  public ShaclService shaclService(@Autowired Shapes defaultApplicationProfile,
      @Value("${shacl.strictModeFiltering}") boolean strictModeFiltering) {
    return new ShaclService(defaultApplicationProfile, strictModeFiltering);
  }

  @Bean
  public Shapes defaultApplicationProfile(@Value("${shacl.application-profile}") Resource applicationProfile)
      throws IOException {
    var shapesGraph = ModelUtils.toModel(applicationProfile.getInputStream(),
        ModelUtils.filenameToLang(applicationProfile.getFilename(), Lang.TURTLE)).getGraph();
    return Shapes.parse(shapesGraph);

  }

}
