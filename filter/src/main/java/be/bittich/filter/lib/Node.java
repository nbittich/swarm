package be.bittich.filter.lib;

import com.fasterxml.jackson.annotation.JsonProperty;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class Node {
  private static final String XML_LANG = "xml:lang";
  private String type;
  private String value;
  private String datatype;
  @JsonProperty(XML_LANG)
  private String language;
}
