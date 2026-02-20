# Add Pattern - Crear un nuevo patron de codigo para el sistema de entrenamiento

Guia interactiva para crear un nuevo patron de codigo en el formato correcto del sistema de entrenamiento MCP.

## Contexto del proyecto

Este MCP server almacena patrones de codigo en `data/patterns/` como archivos JSON. Cada archivo agrupa patrones por framework y sigue este schema:

```json
{
  "patterns": [
    {
      "id": "framework-category-nombre-unico",
      "category": "categoria",
      "framework": "nombre-framework",
      "version": "version",
      "title": "Titulo descriptivo del patron",
      "description": "Descripcion detallada de cuando y por que usar este patron",
      "code": "// Ejemplo de codigo completo y funcional",
      "tags": ["tag1", "tag2"],
      "usage_count": 0,
      "relevance_score": 0.85,
      "created_at": "2025-01-01T00:00:00Z",
      "updated_at": "2025-01-01T00:00:00Z"
    }
  ]
}
```

## Frameworks soportados

- **blazor-server**, **aspnet-core**, **dotnet** (.NET)
- **react**, **vue**, **nextjs**, **express**, **svelte** (Node.js)
- **django**, **flask**, **fastapi** (Python)
- **actix-web**, **axum**, **tokio**, **rust** (Rust)
- **gin**, **fiber**, **go** (Go)
- **spring**, **java** (Java)
- **laravel**, **symfony**, **wordpress**, **php** (PHP)

## Categorias comunes

- `lifecycle` - Ciclo de vida de componentes
- `performance` - Optimizacion y rendimiento
- `security` - Seguridad y validacion
- `error-handling` - Manejo de errores
- `state-management` - Gestion de estado
- `dependency-injection` - Inyeccion de dependencias
- `data-apis` - APIs y acceso a datos
- `testing` - Pruebas y testing
- `authentication` - Autenticacion y autorizacion
- `deployment` - Despliegue y CI/CD

## Proceso

1. **Pregunta al usuario** que framework, categoria y que patron quiere crear (o que describa el patron que tiene en mente)
2. **Determina el archivo destino**: `data/patterns/{framework}-{category}.json` o un archivo existente si ya hay uno para ese framework
3. **Genera el patron** con:
   - `id` unico siguiendo la convencion `{framework}-{category}-{nombre-kebab}`
   - `relevance_score` entre 0.7 y 0.95 segun la utilidad general del patron
   - `code` con un ejemplo completo, funcional y bien comentado
   - `tags` relevantes para facilitar la busqueda
   - Timestamps con la fecha actual en formato ISO 8601
4. **Si el archivo ya existe**, lee su contenido y agrega el nuevo patron al array existente
5. **Si el archivo no existe**, crea uno nuevo con la estructura correcta
6. **Valida** que el JSON resultante es valido
7. **Muestra** el patron creado al usuario para confirmacion

## Reglas de calidad

- El campo `code` debe contener codigo real y funcional, no pseudocodigo
- La `description` debe explicar CUANDO usar el patron y POR QUE es una buena practica
- Los `tags` deben incluir al menos el framework y la categoria
- El `id` debe ser unico globalmente - verifica contra todos los archivos existentes en `data/patterns/`
- El `relevance_score` debe reflejar cuan universalmente aplicable es el patron
