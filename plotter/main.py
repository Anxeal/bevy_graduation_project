import json
import glob
import pandas as pd
import numpy as np

import plotly.graph_objects as go
from plotly.subplots import make_subplots

frames = []
object_counts = []
latencies = []
sizes = []

input_files = glob.glob('*.log')

for input_file in input_files:
    with open(input_file, 'r') as f:
        for line in f:
            data = json.loads(line)
            if 'latency_in_nanos' not in data['fields']:
                continue
            frames.append(data['span']['frame_count'])
            object_counts.append(data['span']['object_count'])
            latencies.append(int(data['fields']['latency_in_nanos']) / 1000000) # convert nanoseconds to milliseconds
            sizes.append(data['fields']['msg_len'] / 1024) # convert bytes to kilobytes

df = pd.DataFrame({ 'Frame': frames, 'Object Count': object_counts, 'Latency (ms)': latencies, 'Size (KB)': sizes})

df = df.groupby(['Frame', 'Object Count']).sum().reset_index()
df = df.drop(columns=['Frame'])

df = df.groupby(['Object Count']).mean().reset_index()
df.sort_values(by='Object Count', inplace=True)

z = np.polyfit(df['Object Count'], df['Latency (ms)'], 1)

fig = make_subplots(specs=[[{"secondary_y": True}]])
fig.add_trace(
    go.Scatter(x=df['Object Count'], y=df['Latency (ms)'], name='Latency (ms)'),
    secondary_y=False,
)
fig.add_trace(
    go.Scatter(x=df['Object Count'], y=z[0] * df['Object Count'] + z[1], name='Line of Best Fit\n(y = {:.5f}x + {:.5f})'.format(z[0], z[1]), mode='lines'),
    secondary_y=False,
)
fig.add_trace(
    go.Scatter(x=df['Object Count'], y=df['Size (KB)'], name='Size (KB)'),
    secondary_y=True,
)


# Add figure title
fig.update_layout(
    title_text="Latency and Size vs. Object Count"
)

# Don't show horizontal grid lines for secondary y-axis
fig.update_layout(yaxis2=dict(showgrid=False))

# Put the legend above the plot
fig.update_layout(legend=dict(
    orientation="h",
    yanchor="bottom",
    y=1.02,
    xanchor="right",
    x=1
))


# Set x-axis title
fig.update_xaxes(title_text="Object Count")

# Set y-axes titles
fig.update_yaxes(title_text="Latency (ms)", secondary_y=False)
fig.update_yaxes(title_text="Size (KB)", secondary_y=True)


# Save the figure as a html file
fig.show()

# Save the figure as a static image
fig.write_image('plot.png')