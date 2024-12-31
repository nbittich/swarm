package be.bittich.filter.lib;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

import com.apicatalog.jsonld.uri.Path;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonMappingException;

import be.bittich.filter.lib.JobModel.Payload;
import be.bittich.filter.lib.JobModel.Task;
import be.bittich.filter.lib.JobModel.TaskResult;
import lombok.extern.slf4j.Slf4j;

@Slf4j
public class JobModelTest {

  @Test
  public void testDeserializeTaskModel() throws JsonMappingException, JsonProcessingException {
    var taskJson = """
                  {
            "_id": "0193f9ecf82c7c4389c64ca0f2f10f48",
            "order": 1,
            "jobId": "0193f9e4c63b7b20bed3f38bcf6fc65d",
            "name": "extract",
            "creationDate": "2024-12-24T18:27:57.100280668Z",
            "modifiedDate": "2024-12-24T18:29:05.070545820Z",
            "payload": {
                "type": "fromPreviousStep",
                "value": {
                    "taskId": "0193f9e4c6487552949231ec26cc8b60",
                    "payload": {
                        "type": "scrapeWebsite",
                        "value": {
                            "successCount": 995,
                            "failureCount": 0,
                            "manifestFilePath": "/share/0193f9e4c63b7b20bed3f38bcf6fc65d/collect/manifest.json"
                        }
                    }
                }
            },
            "result": {
                "type": "extractRDFa",
                "value": {
                    "successCount": 991,
                    "failureCount": 4,
                    "manifestFilePath": "/share/0193f9e4c63b7b20bed3f38bcf6fc65d/extract/manifest.json"
                }
            },
            "hasSubTask": true,
            "status": {
                "type": "success"
            },
            "outputDir": "/share/0193f9e4c63b7b20bed3f38bcf6fc65d/extract"
        }
                    """;
    var task = Constant.MAPPER.readValue(taskJson, Task.class);

    assertTrue(task.payload() instanceof Payload.FromPreviousStep fps
        && fps.value().payload() instanceof TaskResult.ScrapeWebsite sw
        && sw.value().successCount() == 995
        && fps.value().taskId().equals("0193f9e4c6487552949231ec26cc8b60"));
  }

  @Test
  public void testDeserializeJobModel() throws JsonMappingException, JsonProcessingException {
    var jobJson = """
        {
          "_id": "0193fe1f1b407053b214a1d4382e77fa",
          "name": "Harvest",
          "targetUrl": "https://besluitvorming.leuven.be/zittingen/lijst",
          "rootDir": "/share/0193fe1f1b407053b214a1d4382e77fa",
          "creationDate": "2024-12-25T14:01:11.745080142Z",
          "modifiedDate": "2024-12-25T15:24:38.125835972Z",
          "status": {
            "type": "success"
          },
          "definition": {
            "id": "0193e822c0377b8187a6d151dbbb4216",
            "name": "Harvest",
            "allowConcurrentRun": false,
            "tasks": [
              {
                "name": "collect",
                "order": 0,
                "payload": {
                  "type": "scrapeUrl",
                  "value": "https://besluitvorming.leuven.be/zittingen/lijst"
                }
              },
              {
                "name": "extract",
                "order": 1,
                "payload": {
                  "type": "fromPreviousStep",
                  "value": {
                    "taskId": "",
                    "payload": {
                      "type": "scrapeWebsite",
                      "value": {
                        "successCount": 0,
                        "failureCount": 0,
                        "manifestFilePath": ""
                      }
                    }
                  }
                }
              }
            ]
          }
        }

                    """;

    var job = Constant.MAPPER.readValue(jobJson, JobModel.Job.class);
    assertTrue(job.definition().tasks().getLast().payload() instanceof Payload.FromPreviousStep fps
        && fps.value().payload() instanceof TaskResult.ScrapeWebsite sw
        && sw.value().successCount() == 0);
  }

  @Test
  public void serializeAndDeserializePath() throws JsonMappingException, JsonProcessingException {
    var json = """
            {
            "_id": "01940798ff7178038b8cb34de2445ca2",
            "order": 2,
            "jobId": "01940798f67c783190e30419949c8c54",
            "name": "filter",
            "creationDate": "2024-12-27T10:10:54.961268847Z",
            "modifiedDate": "2024-12-27T10:10:55.055384742Z",
            "payload": {
                "type": "fromPreviousStep",
                "value": {
                    "taskId": "01940798ff657cb19394258b534c6a41",
                    "payload": {
                        "type": "extractRDFa",
                        "value": {
                            "successCount": 1,
                            "failureCount": 0,
                            "manifestFilePath": "/share/01940798f67c783190e30419949c8c54/extract/manifest.json"
                        }
                    }
                }
            },
            "result": {
                "type": "filterSHACL",
                "value": {
                    "successCount": 1,
                    "failureCount": 0,
                    "manifestFilePath": "/share/01940798f67c783190e30419949c8c54/filter/manifest.json"
                }
            },
            "hasSubTask": true,
            "status": {
                "type": "success"
            },
            "outputDir": "/share/01940798f67c783190e30419949c8c54/filter/"
        }

            """;
    var task = Constant.MAPPER.readValue(json, Task.class);
    assertEquals(task.outputDir().toString(), Path.of("/share/01940798f67c783190e30419949c8c54/filter").toString());
    json = Constant.MAPPER.writeValueAsString(task);

    log.info("{}", json);
  }
}
