#version 150 core

in vec3 a_Pos;
in vec3 a_Color;
out vec4 v_Color;
out vec3 v_Pos;

layout (std140)
uniform Locals {
    mat4 u_Transform;
};

void main() {
    v_Color = vec4(a_Color, 1.0);
    v_Pos = a_Pos;
    gl_Position = u_Transform * vec4(a_Pos, 1.0);
    gl_ClipDistance[0] = 1.0;
}