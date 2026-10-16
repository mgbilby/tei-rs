<?xml version="1.0" encoding="UTF-8"?>
<!--
  Turns an SVRL report into one tab-separated line per failed assert or
  successful report: kind (assert/report), role (or -), location (an XPath
  with Q{uri}local[n] steps), the context of the rule that fired, and the
  test expression with whitespace normalised. A report with nothing to say
  yields no lines.
-->
<xsl:stylesheet version="2.0"
                xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
                xmlns:svrl="http://purl.oclc.org/dsdl/svrl">
  <xsl:output method="text"/>
  <xsl:template match="/">
    <xsl:for-each select="//svrl:failed-assert | //svrl:successful-report">
      <xsl:value-of select="if (self::svrl:failed-assert) then 'assert' else 'report'"/>
      <xsl:text>&#9;</xsl:text>
      <xsl:value-of select="(@role[normalize-space()], '-')[1]"/>
      <xsl:text>&#9;</xsl:text>
      <xsl:value-of select="@location"/>
      <xsl:text>&#9;</xsl:text>
      <xsl:value-of select="normalize-space(preceding-sibling::svrl:fired-rule[1]/@context)"/>
      <xsl:text>&#9;</xsl:text>
      <xsl:value-of select="normalize-space(@test)"/>
      <xsl:text>&#10;</xsl:text>
    </xsl:for-each>
  </xsl:template>
</xsl:stylesheet>
