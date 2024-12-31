package be.bittich.filter.lib;

import lombok.SneakyThrows;
import lombok.extern.slf4j.Slf4j;
import org.apache.commons.io.IOUtils;
import org.apache.jena.graph.Graph;
import org.apache.jena.graph.Node;
import org.apache.jena.graph.NodeFactory;
import org.apache.jena.graph.Triple;
import org.apache.jena.rdf.model.Model;
import org.apache.jena.rdf.model.ModelFactory;
import org.apache.jena.rdf.model.ResourceFactory;
import org.apache.jena.riot.Lang;
import org.apache.jena.shacl.ShaclValidator;
import org.apache.jena.shacl.Shapes;
import org.apache.jena.shacl.ValidationReport;
import org.apache.jena.shacl.engine.ShaclPaths;
import org.apache.jena.shacl.engine.TargetType;
import org.apache.jena.vocabulary.RDF;

import java.io.File;
import java.io.FileInputStream;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.stream.Collectors;

@Slf4j
public class ShaclService {
  private final Shapes applicationProfile;
  private final boolean strictModeFiltering;

  public ShaclService(Shapes applicationProfile, boolean strictModeFiltering) {
    this.applicationProfile = applicationProfile;
    this.strictModeFiltering = strictModeFiltering;
  }

  public ValidationReport validate(Graph dataGraph) {
    return validate(dataGraph, applicationProfile);
  }

  public Model filter(Model model, ValidationReport report) {
    Model copy = ModelFactory.createDefaultModel();
    copy.add(model);
    Graph dataGraph = copy.getGraph();
    return ModelFactory.createModelForGraph(filter(dataGraph, applicationProfile, report));
  }

  public Model filter(Model model) {
    Graph dataGraph = model.getGraph();
    ValidationReport report = validate(dataGraph);
    return filter(model, report);
  }

  public Graph filter(InputStream dataModel, Lang modelLang) {
    Graph dataGraph = ModelUtils.toModel(dataModel, modelLang).getGraph();
    ValidationReport report = validate(dataGraph);
    return filter(dataGraph, applicationProfile, report);
  }

  @SneakyThrows
  public Graph filter(String dataModel, Lang modelLang) {
    return filter(IOUtils.toInputStream(dataModel, StandardCharsets.UTF_8), modelLang);
  }

  @SneakyThrows
  public Graph filter(File dataModel) {
    return filter(new FileInputStream(dataModel), ModelUtils.filenameToLang(dataModel.getName()));
  }

  @SneakyThrows
  public Graph filter(File dataModel, File shapesFile) {
    return filter(new FileInputStream(dataModel), ModelUtils.filenameToLang(dataModel.getName()),
        new FileInputStream(shapesFile), ModelUtils.filenameToLang(shapesFile.getName()));
  }

  public static List<String> getTargetClasses(Shapes shapes) {
    return shapes.getTargetShapes().stream()
        .map(t -> t.getTargets().stream().filter(s -> s.getTargetType().equals(TargetType.targetClass)).findFirst())
        .filter(java.util.Optional::isPresent)
        .map(j -> j.map(jk -> jk.getObject().getURI()).get())
        .collect(Collectors.toList());
  }

  public Graph filter(Graph dataGraph, Shapes shapes, ValidationReport report) {
    var graphModel = ModelFactory.createModelForGraph(dataGraph);
    List<String> targetClasses = getTargetClasses(shapes);
    log.debug("target classes {}", targetClasses);

    report.getEntries().forEach(r -> {
      Node subject = r.focusNode();
      Node predicate = ShaclPaths.pathNode(r.resultPath());
      if (strictModeFiltering && subject != null && subject.isURI()) {
        graphModel.removeAll(ResourceFactory.createResource(subject.getURI()), null, null);
      } else {
        graphModel.getGraph().remove(subject, predicate, null);
      }
    });

    // filter the classes not defined as target shapes
    Set<String> subjectsDefinedAsTargetClass = new HashSet<>();
    List<Triple> tripleNotDefinedAsTargetClass = new ArrayList<>();

    for (Triple triple : dataGraph
        .find(null, RDF.type.asNode(), null)
        .filterDrop(triple -> triple.getObject() == null || triple.getSubject() == null || !triple.getSubject().isURI()
            || !triple.getObject().isURI())
        .toList()) {

      if (targetClasses.contains(triple.getObject().getURI())) {
        subjectsDefinedAsTargetClass.add(triple.getSubject().getURI());
      } else {

        tripleNotDefinedAsTargetClass.add(triple);
      }

    }
    List<String> classesNotDefinedAsTargetShapes = tripleNotDefinedAsTargetClass.stream()
        .filter(triple -> !subjectsDefinedAsTargetClass.contains(triple.getSubject().getURI()))
        .peek(triple -> log.debug("subject <{}> with class <{}> will be filtered", triple.getSubject().getURI(),
            triple.getObject().getURI()))
        .map(triple -> triple.getSubject().getURI())
        .collect(Collectors.toList());

    log.debug("subject with class not defined: {}", classesNotDefinedAsTargetShapes);
    classesNotDefinedAsTargetShapes.forEach(sub -> dataGraph.remove(NodeFactory.createURI(sub), null, null));

    return graphModel.getGraph();
  }

  public static ValidationReport validate(Graph dataGraph, Shapes shapes) {
    return ShaclValidator.get().validate(shapes, dataGraph);
  }

  public Graph filter(InputStream dataModel, Lang modelLang, InputStream shapesModel, Lang shapesLang) {
    Graph dataGraph = ModelUtils.toModel(dataModel, modelLang).getGraph();
    Graph shapesGraph = ModelUtils.toModel(shapesModel, shapesLang).getGraph();
    Shapes shapes = Shapes.parse(shapesGraph);
    ValidationReport report = validate(dataGraph, shapes);
    return filter(dataGraph, shapes, report);
  }

  public static ValidationReport fromModel(Model report) {
    return ValidationReport.fromModel(report);
  }

  public static ValidationReport fromGraph(Graph report) {
    return ValidationReport.fromGraph(report);
  }

}
