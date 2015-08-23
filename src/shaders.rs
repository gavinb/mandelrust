//----------------------------------------------------------------------------

pub static VERTEX_SHADER_SOURCE: &'static str = "

#version 150

in vec2 position;
in vec2 texcoord;

out vec2 v_texcoord;

void main()
{
    gl_Position = vec4(position, 0.0, 1.0);
    v_texcoord = texcoord;
}
";

//----------------------------------------------------------------------------

pub static FRAGMENT_SHADER_SOURCE: &'static str = "

#version 150

in vec2 v_texcoord;

out vec4 outColor;

uniform sampler2D tex;

void main()
{
    outColor = texture(tex, v_texcoord) * vec4(8,8,8,1);
}
";

//----------------------------------------------------------------------------
