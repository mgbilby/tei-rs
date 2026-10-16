<?xml version="1.0" encoding="UTF-8"?>
<!--
  Extracts the ISO Schematron rules embedded in a RELAX NG schema (XML syntax)
  into a standalone Schematron schema, the way TEI's ODD toolchain does:
  every sch:ns (one per prefix), schema-level sch:let, sch:pattern and
  sch:diagnostics, in document order, with queryBinding="xslt2". Included and
  externally referenced grammars are followed, and Schematron 1.5 elements
  are moved to the ISO namespace. tei-rs's own extraction
  (src/schematron.rs) must stay equivalent to this.

  The schema, each pattern and each schema-level sch:let get an xml:base
  naming the file they come from, which SchXslt carries over to the compiled
  stylesheet: relative URIs in doc() and document() then resolve against the
  schema's location, as in tei-rs, instead of the compiled stylesheet's.

  Usage: java -jar saxon-he.jar -xsl:extract.xsl -s:schema.rng -o:schema.sch
-->
<xsl:stylesheet version="2.0"
                xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
                xmlns:rng="http://relaxng.org/ns/structure/1.0"
                xmlns:sch="http://purl.oclc.org/dsdl/schematron"
                xmlns:s="http://www.ascc.net/xml/schematron"
                exclude-result-prefixes="rng s">

  <xsl:template match="/">
    <xsl:variable name="all" as="element()*">
      <xsl:apply-templates select="/" mode="collect"/>
    </xsl:variable>
    <sch:schema queryBinding="xslt2">
      <xsl:attribute name="xml:base" select="base-uri(/)"/>
      <xsl:for-each-group select="$all[self::sch:ns]" group-by="@prefix">
        <sch:ns prefix="{@prefix}" uri="{@uri}"/>
      </xsl:for-each-group>
      <xsl:apply-templates select="$all[self::sch:let]" mode="based"/>
      <xsl:apply-templates select="$all[self::sch:pattern]" mode="based"/>
      <xsl:if test="$all[self::sch:diagnostics]">
        <sch:diagnostics>
          <xsl:copy-of select="$all[self::sch:diagnostics]/sch:diagnostic"/>
        </sch:diagnostics>
      </xsl:if>
    </sch:schema>
  </xsl:template>

  <!-- Schematron elements are copied whole (patterns keep their own lets). -->
  <xsl:template match="sch:ns | sch:pattern | sch:diagnostics" mode="collect">
    <xsl:sequence select="."/>
  </xsl:template>
  <xsl:template match="sch:let[not(ancestor::sch:pattern)]" mode="collect">
    <xsl:sequence select="."/>
  </xsl:template>
  <xsl:template match="s:ns | s:pattern | s:diagnostics" mode="collect">
    <xsl:apply-templates select="." mode="iso"/>
  </xsl:template>

  <xsl:template match="s:*" mode="iso">
    <xsl:element name="sch:{local-name()}" namespace="http://purl.oclc.org/dsdl/schematron">
      <xsl:if test="not(parent::s:*)">
        <xsl:attribute name="xml:base" select="base-uri(.)"/>
      </xsl:if>
      <xsl:copy-of select="namespace::*, @*"/>
      <xsl:apply-templates select="node()" mode="iso"/>
    </xsl:element>
  </xsl:template>
  <xsl:template match="node()" mode="iso">
    <xsl:copy-of select="."/>
  </xsl:template>

  <!-- A copy that records the file the element comes from. -->
  <xsl:template match="*" mode="based">
    <xsl:copy>
      <xsl:attribute name="xml:base" select="base-uri(.)"/>
      <xsl:copy-of select="@*, node()"/>
    </xsl:copy>
  </xsl:template>

  <xsl:template match="rng:include | rng:externalRef" mode="collect">
    <xsl:apply-templates select="document(@href, .)" mode="collect"/>
    <xsl:apply-templates select="*" mode="collect"/>
  </xsl:template>

  <xsl:template match="@* | text() | comment() | processing-instruction()" mode="collect"/>
  <xsl:template match="* | /" mode="collect">
    <xsl:apply-templates select="node()" mode="collect"/>
  </xsl:template>

</xsl:stylesheet>
