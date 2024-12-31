package be.bittich.filter.lib;

import java.time.Duration;
import java.util.List;

import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Service;

import io.nats.client.Connection;
import io.nats.client.ConsumerContext;
import io.nats.client.JetStreamManagement;
import io.nats.client.api.AckPolicy;
import io.nats.client.api.ConsumerConfiguration;
import io.nats.client.api.DiscardPolicy;
import io.nats.client.api.StreamConfiguration;
import io.nats.client.api.StreamInfo;
import lombok.SneakyThrows;
import lombok.extern.slf4j.Slf4j;

@Slf4j
@Service
public class NatsService {
  private final JetStreamManagement jsm;
  private final Connection nc;
  private @Value("${nats.connection.ackWait}") int ackWait;

  public NatsService(
      Connection nc,
      JetStreamManagement jsm) {
    this.jsm = jsm;
    this.nc = nc;
  }

  @SneakyThrows
  public StreamInfo addStream(String stream, List<String> subjects) {
    return this.jsm.addStream(StreamConfiguration.builder()
        .name(stream)
        .subjects(subjects)
        .maxMessages(100_000)
        .discardPolicy(DiscardPolicy.Old)
        .build());
  }

  @SneakyThrows
  public ConsumerContext createDurableConsumer(String consumerName, StreamInfo stream) {

    var streamName = stream.getConfiguration().getName();
    var sc = this.nc.jetStream().getStreamContext(streamName);

    var consumerConfig = ConsumerConfiguration.builder()
        .name(consumerName)
        .durable(consumerName)
        .ackWait(Duration.ofSeconds(ackWait))
        .ackPolicy(AckPolicy.Explicit)
        .build();
    var cc = sc.createOrUpdateConsumer(consumerConfig);

    return cc;
  }

  @SneakyThrows
  public void publish(String subject, Object payload) {
    var bytes = Constant.MAPPER.writeValueAsBytes(payload);
    this.jsm.jetStream().publishAsync(subject, bytes); // wait for ack?

  }
}
