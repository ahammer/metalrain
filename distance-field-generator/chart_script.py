import plotly.graph_objects as go
import plotly.express as px
import json

# Parse the data
data = {
    "components": [
        {"name": "Input", "items": ["Font File (TTF/OTF)", "Character Set (a-zA-Z0-9)", "Basic Shapes", "Configuration"]}, 
        {"name": "SDF Generation", "items": ["sdf::font module", "sdf::shapes module", "sdf::generator module", "Distance Field Algorithms"]}, 
        {"name": "Atlas Packing", "items": ["atlas::packer module", "Texture Atlas Generation", "Grid-based Packing", "Position Calculation"]}, 
        {"name": "Output Generation", "items": ["atlas::registry module", "Sprite Sheet (PNG)", "Registry (JSON)", "Metadata"]}, 
        {"name": "Output", "items": ["sdf_atlas.png", "sdf_registry.json", "Character Metrics", "Shape Coordinates"]}
    ], 
    "flow": [
        {"from": "Input", "to": "SDF Generation"}, 
        {"from": "SDF Generation", "to": "Atlas Packing"}, 
        {"from": "Atlas Packing", "to": "Output Generation"}, 
        {"from": "Output Generation", "to": "Output"}
    ]
}

# Define colors for each component
colors = ['#1FB8CD', '#DB4545', '#2E8B57', '#5D878F', '#D2BA4C']

# Create figure
fig = go.Figure()

# Define positions for components (horizontal flow)
positions = {
    "Input": (1, 3),
    "SDF Generation": (2.5, 3), 
    "Atlas Packing": (4, 3),
    "Output Generation": (5.5, 3),
    "Output": (7, 3)
}

# Box dimensions
box_width = 0.6
box_height = 0.4

# Add rectangular boxes for each component
for i, component in enumerate(data["components"]):
    name = component["name"]
    x_pos, y_pos = positions[name]
    
    # Add rectangular box
    fig.add_shape(
        type="rect",
        x0=x_pos - box_width/2,
        y0=y_pos - box_height/2,
        x1=x_pos + box_width/2,
        y1=y_pos + box_height/2,
        fillcolor=colors[i],
        line=dict(color="white", width=2)
    )
    
    # Abbreviate component names to fit 15 char limit
    if name == "SDF Generation":
        display_name = "SDF Gen"
    elif name == "Atlas Packing":
        display_name = "Atlas Pack"
    elif name == "Output Generation":
        display_name = "Output Gen"
    else:
        display_name = name
    
    # Add component name text
    fig.add_trace(go.Scatter(
        x=[x_pos],
        y=[y_pos],
        mode='text',
        text=[display_name],
        textposition='middle center',
        textfont=dict(size=12, color='white', family='Arial Black'),
        showlegend=False
    ))

# Add flow arrows between components
for flow in data["flow"]:
    from_pos = positions[flow["from"]]
    to_pos = positions[flow["to"]]
    
    # Calculate arrow positions (from edge of box to edge of box)
    start_x = from_pos[0] + box_width/2
    end_x = to_pos[0] - box_width/2
    
    # Add arrow line
    fig.add_trace(go.Scatter(
        x=[start_x, end_x],
        y=[from_pos[1], to_pos[1]],
        mode='lines',
        line=dict(color='#666666', width=3),
        showlegend=False
    ))
    
    # Add arrowhead
    fig.add_trace(go.Scatter(
        x=[end_x],
        y=[to_pos[1]],
        mode='markers',
        marker=dict(symbol='triangle-right', size=12, color='#666666'),
        showlegend=False
    ))

# Add detail items for each component
for i, component in enumerate(data["components"]):
    items = component["items"]
    x_pos, y_pos = positions[component["name"]]
    
    # Show up to 4 items per component to avoid overcrowding
    display_items = items[:4]
    
    for j, item in enumerate(display_items):
        # Abbreviate items to fit 15 char limit while preserving key info
        if "sdf::font module" in item:
            abbreviated = "sdf::font"
        elif "sdf::shapes module" in item:
            abbreviated = "sdf::shapes"
        elif "sdf::generator module" in item:
            abbreviated = "sdf::generator"
        elif "atlas::packer module" in item:
            abbreviated = "atlas::packer"
        elif "atlas::registry module" in item:
            abbreviated = "atlas::registry"
        elif "Font File (TTF/OTF)" in item:
            abbreviated = "Font Files"
        elif "Character Set (a-zA-Z0-9)" in item:
            abbreviated = "Char Set"
        elif "Distance Field Algorithms" in item:
            abbreviated = "Dist Field Alg"
        elif "Texture Atlas Generation" in item:
            abbreviated = "Atlas Gen"
        elif "Grid-based Packing" in item:
            abbreviated = "Grid Packing"
        elif "Position Calculation" in item:
            abbreviated = "Pos Calc"
        elif "Sprite Sheet (PNG)" in item:
            abbreviated = "Sprite (PNG)"
        elif "Registry (JSON)" in item:
            abbreviated = "Registry JSON"
        elif "Character Metrics" in item:
            abbreviated = "Char Metrics"
        elif "Shape Coordinates" in item:
            abbreviated = "Shape Coords"
        elif len(item) > 15:
            abbreviated = item[:15]
        else:
            abbreviated = item
            
        # Position items below the box with better spacing
        fig.add_trace(go.Scatter(
            x=[x_pos],
            y=[y_pos - 0.7 - (j * 0.25)],
            mode='text',
            text=[abbreviated],
            textposition='middle center',
            textfont=dict(size=9, color=colors[i]),
            showlegend=False
        ))

# Update layout
fig.update_layout(
    title="SDF Generator Project Structure",
    xaxis=dict(showgrid=False, showticklabels=False, zeroline=False, range=[0.2, 7.8]),
    yaxis=dict(showgrid=False, showticklabels=False, zeroline=False, range=[0.5, 4]),
    plot_bgcolor='white',
    showlegend=False
)

fig.update_traces(cliponaxis=False)

# Save the chart
fig.write_image("sdf_generator_flowchart.png", width=1200, height=600)