#version 150 core

in vec4 v_Color;
in vec3 v_Pos;
out vec4 Target0;

void main() {
  float dist = dot(v_Pos, v_Pos);
  float blend = 1.0 - min(dist * 0.3, 1);
  Target0 = v_Color * blend;
}