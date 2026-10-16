FROM python:3.13-slim
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1
WORKDIR /app
COPY . .
RUN useradd --system --uid 10001 omega
USER omega
EXPOSE 8080
CMD ["python","server.py"]
