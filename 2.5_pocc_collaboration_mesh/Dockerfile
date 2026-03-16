# L2.5 Tensor Telepathy Engine — PoCC Wind Tunnel
FROM python:3.11-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies
COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

# Copy source
COPY tensor_telepathy_engine/src/ ./

EXPOSE 50051

CMD ["python", "monte_carlo_smoothing.py"]

LABEL org.opencontainers.image.title="Tensor Wind Tunnel" \
      org.opencontainers.image.description="Life++ L2.5 Monte Carlo Tensor Validator" \
      org.opencontainers.image.source="https://github.com/poccahin/pocc"
