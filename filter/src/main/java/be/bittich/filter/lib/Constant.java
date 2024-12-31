package be.bittich.filter.lib;

import java.nio.file.Path;
import java.util.function.Function;

import com.fasterxml.jackson.databind.Module;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.SerializationFeature;
import com.fasterxml.jackson.databind.module.SimpleModule;
import com.fasterxml.jackson.databind.ser.std.ToStringSerializer;
import com.fasterxml.jackson.datatype.jdk8.Jdk8Module;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;

public interface Constant {
  static String TASK_EVENT_STREAM = "EVENTSTASKS";
  static String SUB_TASK_EVENT_STREAM = "EVENTSSUBTASKS";
  static String TASK_STATUS_CHANGE_SUBJECT = "events.task.status.change.>";
  static String SUB_TASK_STATUS_CHANGE_SUBJECT = "events.subtask.status.change.>";
  static Function<String, String> TASK_STATUS_CHANGE_EVENT = (s) -> String.format("events.task.status.change.%s",
      s);
  static Function<String, String> SUB_TASK_STATUS_CHANGE_EVENT = (s) -> String.format("events.subtask.status.change.%s",
      s);

  static String FILTER_CONSUMER = "filter";
  static ObjectMapper MAPPER = new ObjectMapper()
      .registerModule(new Jdk8Module())
      .registerModule(new JavaTimeModule())
      .registerModule(pathModule())
      .disable(SerializationFeature.WRITE_DATES_AS_TIMESTAMPS);
  static String MANIFEST_FILE_NAME = "manifest.json";

  private static Module pathModule() {
    SimpleModule m = new SimpleModule("PathToString");
    m.addSerializer(Path.class, new ToStringSerializer());
    return m;
  }
}
