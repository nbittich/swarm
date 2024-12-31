package be.bittich.filter.lib;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.io.InputStream;
import java.io.StringWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.time.LocalDateTime;
import java.time.ZoneId;
import java.time.format.DateTimeFormatter;
import java.util.List;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import lombok.SneakyThrows;
import org.apache.commons.io.IOUtils;
import org.apache.commons.lang3.StringUtils;
import org.apache.jena.datatypes.BaseDatatype;
import org.apache.jena.graph.Node;
import org.apache.jena.query.Dataset;
import org.apache.jena.rdf.model.Model;
import org.apache.jena.rdf.model.ModelFactory;
import org.apache.jena.rdf.model.Property;
import org.apache.jena.rdf.model.RDFNode;
import org.apache.jena.rdf.model.Resource;
import org.apache.jena.rdf.model.ResourceFactory;
import org.apache.jena.riot.Lang;
import org.apache.jena.riot.RDFDataMgr;
import org.apache.jena.riot.RDFLanguages;
import org.apache.jena.riot.RiotException;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public interface ModelUtils {
  String DEFAULT_WELL_KNOWN_PREFIX = "http://data.lblod.info/.well-known/genid";
  String CONTENT_TYPE_JSON_LD = "application/ld+json";
  Logger log = LoggerFactory.getLogger(ModelUtils.class);

  static Model toModel(String value, String lang) {
    if (StringUtils.isEmpty(value))
      throw new RuntimeException("model cannot be empty");
    return toModel(IOUtils.toInputStream(value, StandardCharsets.UTF_8), lang);
  }

  static String uuid() {
    return UUIDv7.get();
  }

  static String formattedDate(LocalDateTime ldt) {
    return DateTimeFormatter.ISO_OFFSET_DATE_TIME.format(
        ldt.atZone(ZoneId.systemDefault()));
  }

  static boolean equals(Model firstModel, Model secondModel) {
    return firstModel.isIsomorphicWith(secondModel);
  }

  static Model difference(Model firstModel, Model secondModel) {
    return firstModel.difference(secondModel);
  }

  static Model intersection(Model firstModel, Model secondModel) {
    return firstModel.intersection(secondModel);
  }

  static Model toModel(InputStream is, String lang) {
    try (var stream = is) {
      Model graph = ModelFactory.createDefaultModel();
      graph.read(stream, "", lang);
      return graph;
    } catch (IOException e) {
      throw new RuntimeException(e);
    }
  }

  static Model toModel(InputStream is, Lang lang) {
    return toModel(is, lang.getName());
  }

  static Lang filenameToLang(String filename) {
    return RDFLanguages.filenameToLang(filename);
  }

  static Lang filenameToLang(String filename, Lang fallback) {
    return RDFLanguages.filenameToLang(filename, fallback);
  }

  static String getContentType(String lang) {
    return getContentType(getRdfLanguage(lang));
  }

  static String getContentType(Lang lang) {
    return lang.getContentType().getContentTypeStr();
  }

  static String getExtension(String lang) {
    return getExtension(getRdfLanguage(lang));
  }

  static String getExtension(Lang lang) {
    return lang.getFileExtensions().stream().findFirst().orElse("txt");
  }

  static Lang getRdfLanguage(String lang) {
    return RDFLanguages.nameToLang(lang);
  }

  static String toString(Model model, Lang lang) {
    StringWriter writer = new StringWriter();
    model.write(writer, lang.getName());
    return writer.toString();
  }

  static String toString(Dataset dataset, Lang lang) {
    StringWriter writer = new StringWriter();
    RDFDataMgr.write(writer, dataset, lang);
    return writer.toString();
  }

  static byte[] toBytes(Model model, Lang lang) {
    ByteArrayOutputStream bos = new ByteArrayOutputStream();
    model.write(bos, lang.getName());
    return bos.toByteArray();
  }

  static Model replaceAnonNodes(Model model, String nodePrefix) {
    Model m = ModelFactory.createDefaultModel();
    model.listStatements()
        .toList()
        .stream()
        .map(statement -> {
          var subject = statement.getSubject();
          var predicate = statement.getPredicate();
          var object = statement.getObject();
          if (subject.isAnon()) {
            subject = ResourceFactory.createResource(
                blankNodeToIriString(subject.asNode(), nodePrefix));
          }
          if (predicate.isAnon()) {
            predicate = ResourceFactory.createProperty(
                blankNodeToIriString(predicate.asNode(), nodePrefix));
          }
          if (object.isResource() && object.isAnon()) {
            object = ResourceFactory.createProperty(
                blankNodeToIriString(object.asNode(), nodePrefix));
          }
          return ResourceFactory.createStatement(subject, predicate, object);
        })
        .forEach(m::add);
    return m;
  }

  static Model replaceAnonNodes(Model model) {
    return replaceAnonNodes(model, DEFAULT_WELL_KNOWN_PREFIX);
  }

  static String blankNodeToIriString(Node node, String nodePrefix) {
    if (node.isBlank()) {
      String label = node.getBlankNodeLabel();
      return "%s/%s".formatted(nodePrefix, label);
    }
    if (node.isURI())
      return node.getURI();
    throw new RiotException("Not a blank node or URI");
  }

  static Property nodeToProperty(be.bittich.filter.lib.Node node) {
    if (node.getType().equals("uri")) {
      return ResourceFactory.createProperty(node.getValue());
    } else {
      log.error("Unknown type '{}' for node", node.getType());
      return null;
    }
  }

  static RDFNode nodeToResource(be.bittich.filter.lib.Node node) {
    RDFNode resource = null;
    String type = node.getType();

    switch (type) {
      case "uri":
        resource = ResourceFactory.createResource(node.getValue());
        break;
      case "literal":
        if (StringUtils.isNotEmpty(node.getLanguage())) {
          resource = ResourceFactory.createLangLiteral(node.getValue(),
              node.getLanguage());
        } else if (StringUtils.isNotEmpty(node.getDatatype())) {
          resource = ResourceFactory.createTypedLiteral(
              node.getValue(), new BaseDatatype(node.getDatatype()));
        } else {
          resource = ResourceFactory.createStringLiteral(node.getValue());
        }
        break;
      case "bnode":
        resource = ResourceFactory.createResource();
        break;
      default:
        log.error("Unknown type '{}' for node", node.getType());
    }
    return resource;
  }

  @SneakyThrows
  static File toFile(Model content, Lang rdfLang, String path) {
    var file = new File(path);
    try (var fr = new FileWriter(file)) {
      content.write(new FileWriter(file), rdfLang.getName());
    }
    return file;
  }

  @SneakyThrows
  static File toFile(Model content, Lang rdfLang, Path path) {
    var file = path.toFile();
    try (var fr = new FileWriter(file)) {
      content.write(new FileWriter(file), rdfLang.getName());
    }
    return file;
  }

  static Model merge(Model modelA, Model modelB) {
    return ModelFactory.createUnion(modelA, modelB);
  }

  private static void extractFromModel(Resource subject, Model model,
      Model newModel,
      List<String> statementsProcessed) {
    Model m = model.listStatements(subject, null, (RDFNode) null).toModel();
    newModel.add(m);
    m.listStatements()
        .toList()
        .stream()
        .filter(statement -> statement.getObject().isResource())
        .map(statement -> statement.getObject().asResource())
        .filter(resource -> !statementsProcessed.contains(resource.getURI()))
        .forEach(s -> extractFromModel(s, model, newModel,
            Stream
                .concat(statementsProcessed.stream(),
                    Stream.of(s.getURI()))
                .collect(Collectors.toList())));
  }

  /**
   * Extract all the triples linked to a subject from a model to a new model.
   * This method is very handy if you are just interested by a specific part of
   * a graph
   * 
   * @param subject
   * @param model
   * @param newModel
   */
  static void extractFromModel(Resource subject, Model model, Model newModel) {
    extractFromModel(subject, model, newModel, List.of());
  }

  static Model extractFromModel(Resource subject, Model model) {
    var newModel = ModelFactory.createDefaultModel();
    extractFromModel(subject, model, newModel, List.of());
    return newModel;
  }
}
