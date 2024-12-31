package be.bittich.filter.lib;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.annotation.JsonTypeInfo;
import com.fasterxml.jackson.annotation.JsonSubTypes;
import com.fasterxml.jackson.databind.JsonNode;

import lombok.Builder;

import java.nio.file.Path;
import java.time.OffsetDateTime;
import java.util.List;
import java.util.Optional;

public class JobModel {
  @Builder(toBuilder = true)
  record Job(
      @JsonProperty("_id") String id,
      @JsonProperty("name") String name,
      @JsonProperty("targetUrl") Optional<String> targetUrl,
      @JsonProperty("rootDir") Path rootDir,
      @JsonProperty("creationDate") OffsetDateTime creationDate,
      @JsonProperty("modifiedDate") Optional<OffsetDateTime> modifiedDate,
      @JsonProperty("status") Status status,
      @JsonProperty("definition") JobDefinition definition) {
  }

  @Builder(toBuilder = true)
  public record JobDefinition(
      @JsonProperty("id") String id,
      @JsonProperty("name") String name,
      @JsonProperty("allowConcurrentRun") boolean allowConcurrentRun,
      @JsonProperty("tasks") List<TaskDefinition> tasks) {
  }

  @Builder(toBuilder = true)
  public record TaskDefinition(
      @JsonProperty("name") String name,
      @JsonProperty("order") int order,
      @JsonProperty("payload") Payload payload) {
  }

  @Builder(toBuilder = true)
  public record Task(
      @JsonProperty("_id") String id,
      @JsonProperty("order") int order,
      @JsonProperty("jobId") String jobId,
      @JsonProperty("name") String name,
      @JsonProperty("creationDate") OffsetDateTime creationDate,
      @JsonProperty("modifiedDate") Optional<OffsetDateTime> modifiedDate,
      @JsonProperty("payload") Payload payload,
      @JsonProperty("result") Optional<TaskResult> result,
      @JsonProperty("hasSubTask") boolean hasSubTask,
      @JsonProperty("status") Status status,
      @JsonProperty("outputDir") Path outputDir) {
  }

  @JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.PROPERTY, property = "type")
  @JsonSubTypes({
      @JsonSubTypes.Type(value = Payload.None.class, name = "none"),
      @JsonSubTypes.Type(value = Payload.ScrapeUrl.class, name = "scrapeUrl"),
      @JsonSubTypes.Type(value = Payload.FromPreviousStep.class, name = "fromPreviousStep")
  })
  public sealed interface Payload permits Payload.None, Payload.ScrapeUrl, Payload.FromPreviousStep {

    @Builder(toBuilder = true)
    public record None() implements Payload {
    }

    @Builder(toBuilder = true)
    public record ScrapeUrl(@JsonProperty("value") String value) implements Payload {
    }

    @Builder(toBuilder = true)
    public record FromPreviousStep(
        @JsonProperty("value") FromPreviousStepValue value) implements Payload {
      @Builder(toBuilder = true)
      public record FromPreviousStepValue(
          @JsonProperty("taskId") String taskId,
          @JsonProperty("payload") TaskResult payload) {
      }
    }
  }

  @JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.PROPERTY, property = "type")
  @JsonSubTypes({
      @JsonSubTypes.Type(value = TaskResult.ScrapeWebsite.class, name = "scrapeWebsite"),
      @JsonSubTypes.Type(value = TaskResult.ExtractRDFa.class, name = "extractRDFa"),
      @JsonSubTypes.Type(value = TaskResult.ComplementWithUuid.class, name = "complementWithUuid"),
      @JsonSubTypes.Type(value = TaskResult.Diff.class, name = "diff"),
      @JsonSubTypes.Type(value = TaskResult.Publish.class, name = "publish"),
      @JsonSubTypes.Type(value = TaskResult.FilterSHACL.class, name = "filterSHACL"),
      @JsonSubTypes.Type(value = TaskResult.Json.class, name = "json")
  })
  public sealed interface TaskResult
      permits TaskResult.ScrapeWebsite, TaskResult.Publish, TaskResult.ExtractRDFa, TaskResult.Json,
      TaskResult.FilterSHACL,
      TaskResult.Diff,
      TaskResult.ComplementWithUuid {
    @Builder(toBuilder = true)
    public record ScrapeWebsite(@JsonProperty("value") ScrapeWebsiteValue value) implements TaskResult {
      @Builder(toBuilder = true)
      public record ScrapeWebsiteValue(@JsonProperty("successCount") int successCount,
          @JsonProperty("failureCount") int failureCount,
          @JsonProperty("manifestFilePath") Path manifestFilePath) {
      }
    }

    @Builder(toBuilder = true)
    public record ExtractRDFa(@JsonProperty("value") ExtractRDFaValue value) implements TaskResult {

      @Builder(toBuilder = true)
      public record ExtractRDFaValue(@JsonProperty("successCount") int successCount,
          @JsonProperty("failureCount") int failureCount,
          @JsonProperty("manifestFilePath") Path manifestFilePath) {
      }
    }

    @Builder(toBuilder = true)
    public record Publish(@JsonProperty("value") PublishValue value) implements TaskResult {

      @Builder(toBuilder = true)
      public record PublishValue(
          @JsonProperty("removedTripleFilePath") Path removedTripleFilePath,
          @JsonProperty("intersectTripleFilePath") Path intersectTripleFilePath,
          @JsonProperty("failedQueryFilePath") Path failedQueryFilePath,
          @JsonProperty("insertedTripleFilePath") Path insertedTripleFilePath) {
      }
    }

    @Builder(toBuilder = true)
    public record Diff(@JsonProperty("value") DiffValue value) implements TaskResult {

      @Builder(toBuilder = true)
      public record DiffValue(@JsonProperty("successCount") int successCount,
          @JsonProperty("failureCount") int failureCount,
          @JsonProperty("manifestFilePath") Path manifestFilePath) {
      }
    }

    @Builder(toBuilder = true)
    public record FilterSHACL(@JsonProperty("value") FilterSHACLValue value) implements TaskResult {
      @Builder(toBuilder = true)
      public record FilterSHACLValue(@JsonProperty("successCount") int successCount,
          @JsonProperty("failureCount") int failureCount,
          @JsonProperty("manifestFilePath") Path manifestFilePath) {
      }
    }

    @Builder(toBuilder = true)
    public record ComplementWithUuid(@JsonProperty("value") ComplementWithUuidValue value) implements TaskResult {
      @Builder(toBuilder = true)
      public record ComplementWithUuidValue(@JsonProperty("successCount") int successCount,
          @JsonProperty("failureCount") int failureCount,
          @JsonProperty("manifestFilePath") Path manifestFilePath) {
      }
    }

    @Builder(toBuilder = true)
    public record Json(@JsonProperty("value") JsonNode value) implements TaskResult {
    }
  }

  @Builder(toBuilder = true)
  public record SubTask(
      @JsonProperty("_id") String id,
      @JsonProperty("taskId") String taskId,
      @JsonProperty("creationDate") OffsetDateTime creationDate,
      @JsonProperty("modifiedDate") Optional<OffsetDateTime> modifiedDate,
      @JsonProperty("status") Status status,
      @JsonProperty("result") Optional<SubTaskResult> result) {
  }

  @JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.PROPERTY, property = "type")
  @JsonSubTypes({
      @JsonSubTypes.Type(value = SubTaskResult.ScrapeUrl.class, name = "scrapeUrl"),
      @JsonSubTypes.Type(value = SubTaskResult.NTriple.class, name = "nTriple"),
      @JsonSubTypes.Type(value = SubTaskResult.Json.class, name = "json")
  })
  public sealed interface SubTaskResult
      permits SubTaskResult.Diff, SubTaskResult.ScrapeUrl, SubTaskResult.NTriple, SubTaskResult.Json {
    @Builder(toBuilder = true)
    public record ScrapeUrl(@JsonProperty("value") ScrapeResult value) implements SubTaskResult {
    }

    @Builder(toBuilder = true)
    public record NTriple(@JsonProperty("value") NTripleResult value) implements SubTaskResult {
    }

    @Builder(toBuilder = true)
    public record Diff(@JsonProperty("value") DiffResult value) implements SubTaskResult {
    }

    @Builder(toBuilder = true)
    public record Json(@JsonProperty("value") JsonNode value) implements SubTaskResult {
    }
  }

  @Builder(toBuilder = true)
  public record ScrapeResult(
      @JsonProperty("baseUrl") String baseUrl,
      @JsonProperty("path") Path path,
      @JsonProperty("creationDate") OffsetDateTime creationDate) {
  }

  @Builder(toBuilder = true)
  public record DiffResult(
      @JsonProperty("baseUrl") String baseUrl,
      @JsonProperty("newInsertPath") Optional<Path> newInsertPath,
      @JsonProperty("intersectPath") Optional<Path> intersectPath,
      @JsonProperty("toRemovePath") Optional<Path> toRemovePath,
      @JsonProperty("creationDate") OffsetDateTime creationDate) {
  }

  @Builder(toBuilder = true)
  public record NTripleResult(
      @JsonProperty("baseUrl") String baseUrl,
      @JsonProperty("len") long len,
      @JsonProperty("path") Path path,
      @JsonProperty("creationDate") OffsetDateTime creationDate) {
  }

  @JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.PROPERTY, property = "type")
  @JsonSubTypes({
      @JsonSubTypes.Type(value = Status.Pending.class, name = "pending"),
      @JsonSubTypes.Type(value = Status.Scheduled.class, name = "scheduled"),
      @JsonSubTypes.Type(value = Status.Busy.class, name = "busy"),
      @JsonSubTypes.Type(value = Status.Success.class, name = "success"),
      @JsonSubTypes.Type(value = Status.Failed.class, name = "failed")
  })
  public sealed interface Status permits Status.Pending, Status.Scheduled, Status.Busy, Status.Success, Status.Failed {
    @Builder(toBuilder = true)
    public record Pending() implements Status {
    }

    @Builder(toBuilder = true)
    public record Scheduled() implements Status {
    }

    @Builder(toBuilder = true)
    public record Busy() implements Status {
    }

    @Builder(toBuilder = true)
    public record Success() implements Status {
    }

    @Builder(toBuilder = true)
    public record Failed(@JsonProperty("value") List<String> value) implements Status {
    }
  }

}
