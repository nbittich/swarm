package be.bittich.filter;

import java.io.BufferedReader;
import java.io.FileInputStream;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.IOException;
import java.nio.file.Path;
import java.time.Duration;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.util.HashMap;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.Executors;

import org.apache.jena.riot.Lang;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.boot.CommandLineRunner;
import org.springframework.stereotype.Component;
import org.springframework.util.FileSystemUtils;

import be.bittich.filter.lib.NatsService;
import be.bittich.filter.lib.ShaclService;
import be.bittich.filter.lib.UUIDv7;
import be.bittich.filter.lib.JobModel.NTripleResult;
import be.bittich.filter.lib.JobModel.Payload;
import be.bittich.filter.lib.JobModel.Status;
import be.bittich.filter.lib.JobModel.SubTask;
import be.bittich.filter.lib.JobModel.SubTaskResult;
import be.bittich.filter.lib.JobModel.Task;
import be.bittich.filter.lib.JobModel.TaskResult;
import be.bittich.filter.lib.JobModel.TaskResult.ExtractRDFa;
import be.bittich.filter.lib.JobModel.TaskResult.ExtractRDFa.ExtractRDFaValue;
import be.bittich.filter.lib.JobModel.TaskResult.FilterSHACL.FilterSHACLValue;
import lombok.Builder;
import lombok.SneakyThrows;
import lombok.extern.slf4j.Slf4j;
import be.bittich.filter.lib.Constant;
import be.bittich.filter.lib.ModelUtils;

@Component
@Slf4j
public class AppRunner implements CommandLineRunner {

    @Value("${spring.application.name}")
    private String applicationName;

    private final NatsService nc;
    private final ShaclService shaclService;

    public AppRunner(NatsService nc, ShaclService ss) {
        this.nc = nc;
        this.shaclService = ss;
    }

    @Override
    public void run(String... args) throws Exception {
        var taskEventStream = this.nc.addStream(
                Constant.TASK_EVENT_STREAM, List.of(
                        Constant.TASK_STATUS_CHANGE_SUBJECT));
        this.nc.addStream(Constant.SUB_TASK_EVENT_STREAM,
                List.of(Constant.SUB_TASK_STATUS_CHANGE_SUBJECT));
        var taskEventConsumer = nc.createDurableConsumer(Constant.FILTER_CONSUMER,
                taskEventStream);

        log.info("app {} started and ready to consume messages.", applicationName);

        while (true) {
            var message = taskEventConsumer.next();
            if (message != null) {
                log.debug("received {}", new String(message.getData()));
                try {
                    var task = Constant.MAPPER.readValue(message.getData(), Task.class);
                    if (task.status() instanceof Status.Scheduled
                            && task.payload() instanceof Payload.FromPreviousStep fps
                            && fps.value().payload() instanceof ExtractRDFa er
                            && er.value() instanceof ExtractRDFaValue) {

                        log.debug("handling task");
                        Thread.startVirtualThread(() -> {
                            message.ack();
                            var newTask = task.toBuilder()
                                    .status(Status.Busy.builder().build())
                                    .modifiedDate(Optional.of(OffsetDateTime.now(ZoneOffset.systemDefault())))
                                    .hasSubTask(true).build();
                            nc.publish(Constant.TASK_STATUS_CHANGE_EVENT.apply(newTask.id()), newTask);

                            try {
                                var res = handleTask(nc, newTask);
                                res.ifPresent(
                                        updatedTask -> nc.publish(
                                                Constant.TASK_STATUS_CHANGE_EVENT.apply(updatedTask.id()),
                                                updatedTask));
                            } catch (Exception ex) {
                                log.debug("error {}", ex);
                                newTask = newTask.toBuilder()
                                        .status(Status.Failed.builder()
                                                .value(List.of(String.format("unexpected error %s", ex)))
                                                .build())
                                        .build();
                                nc.publish(Constant.TASK_STATUS_CHANGE_EVENT.apply(newTask.id()), newTask);
                            }
                        });
                    } else {
                        log.debug("no op task {}", task);
                        message.ack();
                    }
                } catch (Exception e) {
                    log.error("could not extract task", e);
                    message.ack();
                }
            }
        }
    }

    public Optional<Task> handleTask(NatsService nc, Task task) throws IOException {
        if (task.payload() instanceof Payload.FromPreviousStep fps
                && fps.value().payload() instanceof ExtractRDFa er
                && er.value() instanceof ExtractRDFaValue payload) {

            if (task.outputDir().toFile().exists()) {
                FileSystemUtils.deleteRecursively(task.outputDir());
            }
            task.outputDir().toFile().mkdirs();
            var successCount = 0;
            var failureCount = 0;
            var handles = new HashMap<SubTask, CompletableFuture<NTripleResult>>();
            var executorService = Executors.newVirtualThreadPerTaskExecutor();
            var outputDir = task.outputDir();
            log.debug("manifest file {}", payload.manifestFilePath());
            try (var manifestReader = new BufferedReader(new FileReader(payload.manifestFilePath().toFile()))) {

                var line = manifestReader.readLine();
                log.debug("reading line {}", line);
                while (line != null) {
                    if (line.trim().isEmpty()) {
                        line = manifestReader.readLine();
                        continue;
                    }
                    var subTask = SubTask.builder().id(UUIDv7.get())
                            .taskId(task.id())
                            .creationDate(OffsetDateTime.now(ZoneOffset.systemDefault()))
                            .status(Status.Busy.builder().build())
                            .build();

                    nc.publish(Constant.SUB_TASK_STATUS_CHANGE_EVENT.apply(subTask.id()), subTask);
                    var lineCopy = line.toString();

                    handles.put(subTask, CompletableFuture.supplyAsync(() -> validateAndFilter(lineCopy, outputDir),
                            executorService));
                    sleep(5); // sleep just a little to avoid consuming all the cpu
                    line = manifestReader.readLine();

                }
                for (var entry : handles.entrySet()) {
                    SubTask subTask = entry.getKey().toBuilder()
                            .modifiedDate(Optional.of(OffsetDateTime.now(ZoneOffset.systemDefault())))
                            .build();
                    try {
                        var res = entry.getValue().get();
                        if (res.len() == 0) {
                            subTask = subTask.toBuilder()
                                    .status(Status.Failed.builder().value(List.of("filtered all data")).build())
                                    .build();
                            failureCount += 1;
                        } else {
                            appendEntryManifestFile(task.outputDir(), res);
                            successCount += 1;
                            subTask = subTask.toBuilder().status(Status.Success.builder().build())
                                    .result(Optional.of(SubTaskResult.NTriple.builder().value(res).build()))
                                    .build();

                        }

                    } catch (InterruptedException | ExecutionException exc) {
                        log.debug("error {}", exc);
                        if (exc.getCause() instanceof ValidateAndFilterException ex) {
                            subTask = subTask.toBuilder()
                                    .status(
                                            Status.Failed.builder()
                                                    .value(List.of(
                                                            "error during extraction! " + ex.exception.getMessage()))
                                                    .build())
                                    .result(Optional.of(SubTaskResult.NTriple.builder()
                                            .value(NTripleResult.builder()
                                                    .len(0)
                                                    .creationDate(OffsetDateTime.now(ZoneOffset.systemDefault()))
                                                    .baseUrl(ex.baseUrl).path(ex.path).build())
                                            .build()))
                                    .build();
                        } else {
                            subTask = subTask.toBuilder()
                                    .status(
                                            Status.Failed.builder()
                                                    .value(List.of(
                                                            "unexpected error during extraction! " + exc.getMessage()))
                                                    .build())
                                    .build();
                        }
                        failureCount += 1;
                    }
                    nc.publish(Constant.SUB_TASK_STATUS_CHANGE_EVENT.apply(subTask.id()), subTask);
                }
                task = task.toBuilder().modifiedDate(Optional.of(OffsetDateTime.now(ZoneOffset.systemDefault())))
                        .build();
                if (successCount == 0 && failureCount > 0) {
                    task = task.toBuilder().status(Status.Failed.builder()
                            .value(
                                    List.of(String.format("task did not succeed: success: %s, failure: %s",
                                            successCount, failureCount)))
                            .build()).build();
                } else {
                    task = task.toBuilder()
                            .result(Optional.of(
                                    TaskResult.FilterSHACL.builder()
                                            .value(
                                                    FilterSHACLValue.builder()
                                                            .successCount(successCount)
                                                            .failureCount(failureCount)
                                                            .manifestFilePath(task.outputDir()
                                                                    .resolve(Constant.MANIFEST_FILE_NAME))
                                                            .build())
                                            .build()))
                            .status(Status.Success.builder().build()).build();
                }
                return Optional.of(task);
            }
        } else {
            log.debug("could not pattern match task");
        }

        return Optional.empty();
    }

    @SneakyThrows
    public void appendEntryManifestFile(Path dirPath, NTripleResult pageRes) {
        var line = Constant.MAPPER.writeValueAsString(pageRes) + "\n";
        var path = dirPath.resolve(Constant.MANIFEST_FILE_NAME);
        var f = path.toFile();
        try (var manifestFile = new FileWriter(f, true)) {
            manifestFile.write(line);
        }

    }

    @Builder
    static class ValidateAndFilterException extends RuntimeException {
        public String baseUrl;
        public Path path;
        public Exception exception;
    }

    @Builder
    static class ParseException extends RuntimeException {
        public Exception exception;
    }

    @SneakyThrows
    NTripleResult parse(String line) {
        return Constant.MAPPER.readValue(line, NTripleResult.class);
    }

    public NTripleResult validateAndFilter(String line, Path outputDir) {
        var payload = parse(line);
        try {
            var model = ModelUtils.toModel(new FileInputStream(payload.path().toFile()),
                    ModelUtils.filenameToLang(payload.path().toFile().getName(), Lang.TURTLE));
            var report = this.shaclService.validate(model.getGraph());
            log.debug("{} is conforms: {}", payload.path(), report.conforms());
            var filtered = this.shaclService.filter(model, report);

            var id = UUIDv7.get();
            var path = outputDir.resolve(String.format("valid-%s.ttl", id));
            ModelUtils.toFile(filtered, Lang.NTRIPLES, path);
            return NTripleResult.builder()
                    .baseUrl(payload.baseUrl())
                    .len(filtered.size())
                    .path(path)
                    .creationDate(OffsetDateTime.now(ZoneOffset.systemDefault()))
                    .build();
        } catch (Throwable ex) {
            throw ValidateAndFilterException.builder().exception(new RuntimeException(ex)).baseUrl(payload.baseUrl())
                    .path(payload.path()).build();
        }

    }

    public void sleep(long millis) {
        try {
            Thread.sleep(Duration.ofMillis(millis));
        } catch (Exception ex) {

        }
    }
}
