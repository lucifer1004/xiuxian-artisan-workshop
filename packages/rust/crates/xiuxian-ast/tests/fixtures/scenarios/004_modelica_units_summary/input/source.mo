within Modelica;
package Units "Library of type and unit definitions"
  extends Modelica.Icons.Package;

package UsersGuide "User's Guide of Units Library"
  extends Modelica.Icons.Information;

  class HowToUseUnits "How to use Units"
    extends Modelica.Icons.Information;

    annotation (DocumentationClass=true, Documentation(info="<html>
<p>
When implementing a Modelica model, every variable needs to
be declared. Physical variables should be declared with a unit.
The basic approach in Modelica is that the unit attribute of
a variable is the <strong>unit</strong> in which the <strong>equations</strong> are <strong>written</strong>,
for example:
</p>

<blockquote><pre>
<strong>model</strong> MassOnGround
  <strong>parameter</strong> Real m(quantity=\"Mass\", unit=\"kg\") \"Mass\";
  <strong>parameter</strong> Real f(quantity=\"Force\", unit=\"N\") \"Driving force\";
  Real s(unit=\"m\") \"Position of mass\";
  Real v(unit=\"m/s\") \"Velocity of mass\";
<strong>equation</strong>
  <strong>der</strong>(s) = v;
  m*<strong>der</strong>(v) = f;
<strong>end</strong> MassOnGround;
</pre></blockquote>

</html>"));

  end HowToUseUnits;

  class Conventions "Conventions"
    extends Modelica.Icons.Information;

    annotation (DocumentationClass=true, Documentation(info="<html>
<p>The following conventions are used in package <code>Modelica.Units.SI</code>:</p>
<ul>
<li>Modelica quantity names are defined according to the recommendations
    of ISO 31.</li>
<li>Modelica units are defined according to the SI base units without
    multiples (only exception \"kg\").</li>
</ul>
</html>"));

  end Conventions;

  class Literature "Literature"
    extends Modelica.Icons.References;

    annotation (Documentation(info="<html>
<p> This package is based on the following references
</p>

<dl>
<dt>ISO 31-1992:</dt>
<dd> <strong>General principles concerning
    quantities, units and symbols</strong>.<br>&nbsp;</dd>
</dl>

</html>"));
  end Literature;

  class Contact "Contact"
    extends Modelica.Icons.Contact;

    annotation (Documentation(info="<html>
<h4>Main author</h4>

<p>
<a href=\"http://www.robotic.dlr.de/Martin.Otter/\"><strong>Martin Otter</strong></a><br>
Deutsches Zentrum f&uuml;r Luft- und Raumfahrt (DLR)<br>
Germany<br>
</p>
</html>"));
  end Contact;
  annotation (DocumentationClass=true, Documentation(info="<html>
<p>
Library <strong>Units</strong> is a <strong>free</strong> Modelica package providing
predefined types, such as <em>Mass</em>,
<em>Length</em>, <em>Time</em>.</p>
</html>"));
end UsersGuide;

  package SI "Library of SI unit definitions"
    extends Modelica.Icons.Package;

    // Space and Time (chapter 1 of ISO 31-1992)

    type Angle = Real (
        final quantity="Angle",
        final unit="rad",
        displayUnit="deg");
    type SolidAngle = Real (final quantity="SolidAngle", final unit="sr");
    type Length = Real (final quantity="Length", final unit="m");
    type PathLength = Length;
    type Position = Length;
    type Distance = Length (min=0);
    type Area = Real (final quantity="Area", final unit="m2");
    type Volume = Real (final quantity="Volume", final unit="m3");
    type Time = Real (final quantity="Time", final unit="s");
    type Duration = Time;
    type AngularVelocity = Real (final quantity="AngularVelocity", final unit="rad/s");
    type Velocity = Real (final quantity="Velocity", final unit="m/s");

    annotation (Documentation(info="<html>
<p>This package provides SI unit type definitions.</p>
</html>"));
  end SI;

annotation (Documentation(info="<html>
<p>Units library root package.</p>
</html>"));
end Units;
