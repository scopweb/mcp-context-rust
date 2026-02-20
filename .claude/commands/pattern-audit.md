# Pattern Audit - Auditoria de calidad de patrones

Analiza todos los patrones en `data/patterns/` y genera un reporte de calidad, completitud y sugerencias de mejora.

## Proceso de auditoria

### 1. Inventario de archivos
Lee todos los archivos `*.json` en `data/patterns/` y lista:
- Nombre del archivo
- Numero de patrones en cada archivo
- Frameworks cubiertos
- Categorias cubiertas

### 2. Validacion de schema
Para cada patron, verifica que tiene todos los campos requeridos:
- `id` (string, no vacio, unico globalmente)
- `category` (string, no vacio)
- `framework` (string, no vacio, debe ser un framework conocido)
- `version` (string)
- `title` (string, no vacio)
- `description` (string, minimo 20 caracteres)
- `code` (string, minimo 10 caracteres, debe contener codigo real)
- `tags` (array de strings, minimo 1 tag)
- `usage_count` (number >= 0)
- `relevance_score` (number entre 0.0 y 1.0)
- `created_at` (string, formato ISO 8601)
- `updated_at` (string, formato ISO 8601)

### 3. Analisis de calidad del codigo
Para cada patron evalua:
- **Completitud**: El `code` es un ejemplo funcional completo o un fragmento incompleto?
- **Documentacion**: La `description` explica el por que, no solo el que?
- **Tags**: Los tags son suficientes para busqueda? Incluyen framework y categoria?
- **Relevance score**: Es coherente con la utilidad real del patron?

### 4. Analisis de cobertura
Genera una matriz de cobertura frameworks vs categorias:

| Framework | lifecycle | performance | security | error-handling | state | DI | data-apis |
|-----------|-----------|-------------|----------|----------------|-------|----|-----------|
| blazor-server | X | X | X | - | X | X | X |
| react | - | - | - | - | - | - | - |
| laravel | - | - | - | - | - | - | - |
| ... | ... | ... | ... | ... | ... | ... | ... |

Identifica los huecos mas criticos.

### 5. Deteccion de problemas
Busca:
- IDs duplicados entre archivos
- Patrones sin tags
- Descriptions demasiado cortas
- Code samples vacios o con solo comentarios
- Relevance scores fuera del rango razonable (< 0.5 o = 1.0)
- Frameworks no reconocidos por el servidor

### 6. Sugerencias de mejora
Basado en el analisis, sugiere:
- Patrones faltantes para los frameworks mas populares
- Categorias subrepresentadas
- Patrones existentes que necesitan mejoras
- Nuevos frameworks que deberian tener cobertura

## Formato del reporte

```
# Pattern Audit Report

## Resumen
- Total de archivos: X
- Total de patrones: X
- Frameworks cubiertos: X
- Categorias cubiertas: X
- Problemas encontrados: X

## Problemas (por severidad)
### Criticos
...
### Warnings
...
### Info
...

## Matriz de cobertura
[tabla]

## Top 5 sugerencias de mejora
1. ...
2. ...
```
