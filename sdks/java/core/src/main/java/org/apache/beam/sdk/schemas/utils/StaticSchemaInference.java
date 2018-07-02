/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package org.apache.beam.sdk.schemas.utils;

import static com.google.common.base.Preconditions.checkArgument;

import com.google.common.collect.ImmutableMap;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.ParameterizedType;
import java.math.BigDecimal;
import java.nio.ByteBuffer;
import java.util.Arrays;
import java.util.Collection;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.function.Function;
import javax.annotation.Nullable;
import org.apache.beam.sdk.schemas.Schema;
import org.apache.beam.sdk.schemas.Schema.FieldType;
import org.apache.beam.sdk.values.TypeDescriptor;
import org.joda.time.ReadableInstant;

/** A set of utilities for inferring a Beam {@link Schema} from static Java types. */
public class StaticSchemaInference {
  enum MethodType {
    GETTER,
    SETTER
  }

  private static final Map<Class, FieldType> PRIMITIVE_TYPES =
      ImmutableMap.<Class, FieldType>builder()
          .put(Byte.class, FieldType.BYTE)
          .put(byte.class, FieldType.BYTE)
          .put(Short.class, FieldType.INT16)
          .put(short.class, FieldType.INT16)
          .put(Integer.class, FieldType.INT32)
          .put(int.class, FieldType.INT32)
          .put(Long.class, FieldType.INT64)
          .put(long.class, FieldType.INT64)
          .put(Float.class, FieldType.FLOAT)
          .put(float.class, FieldType.FLOAT)
          .put(Double.class, FieldType.DOUBLE)
          .put(double.class, FieldType.DOUBLE)
          .put(Boolean.class, FieldType.BOOLEAN)
          .put(boolean.class, FieldType.BOOLEAN)
          .put(BigDecimal.class, FieldType.DECIMAL)
          .build();

  /** Relevant information about a Java type. */
  public static class TypeInformation {
    private String name;
    private TypeDescriptor type;
    private boolean nullable;

    /** Construct a {@link TypeInformation}. */
    public TypeInformation(String name, TypeDescriptor type, boolean nullable) {
      this.name = name;
      this.type = type;
      this.nullable = nullable;
    }

    /** Construct a {@link TypeInformation} from a class member variable. */
    public TypeInformation(Field field) {
      this(
          field.getName(),
          TypeDescriptor.of(field.getGenericType()),
          field.getAnnotation(Nullable.class) != null);
    }

    /** Construct a {@link TypeInformation} from a class getter or setter. */
    public TypeInformation(Method method, MethodType methodType) {
      switch (methodType) {
        case GETTER:
          if (method.getName().startsWith("get")) {
            this.name = ReflectUtils.stripPrefix(method.getName(), "get");
          } else if (method.getName().startsWith("is")) {
            this.name = ReflectUtils.stripPrefix(method.getName(), "is");
          } else {
            throw new RuntimeException("Getter has wrong prefix " + method.getName());
          }
          this.type = TypeDescriptor.of(method.getGenericReturnType());
          this.nullable = method.getAnnotation(Nullable.class) != null;
          break;
        case SETTER:
          if (method.getName().startsWith("set")) {
            this.name = ReflectUtils.stripPrefix(method.getName(), "get");
          } else {
            throw new RuntimeException("Setter has wrong prefix " + method.getName());
          }
          if (method.getParameterCount() != 1) {
            throw new RuntimeException("Setter methods should take a single argument.");
          }
          this.type = TypeDescriptor.of(method.getGenericParameterTypes()[0]);
          this.nullable =
              Arrays.stream(method.getParameterAnnotations()[0])
                  .anyMatch(Nullable.class::isInstance);
      }
    }

    public String getName() {
      return name;
    }

    public TypeDescriptor getType() {
      return type;
    }

    public boolean isNullable() {
      return nullable;
    }

    @Override
    public boolean equals(Object o) {
      if (this == o) {
        return true;
      }
      if (o == null || getClass() != o.getClass()) {
        return false;
      }
      TypeInformation that = (TypeInformation) o;
      return nullable == that.nullable
          && Objects.equals(name, that.name)
          && Objects.equals(type, that.type);
    }

    @Override
    public int hashCode() {
      return Objects.hash(name, type, nullable);
    }
  }

  /**
   * Infer a schema from a Java class.
   *
   * <p>Takes in a function to extract a list of field types from a class. Different callers may
   * have different strategies for extracting this list: e.g. introspecting public member variables,
   * public getter methods, or special annotations on the class.
   */
  public static Schema schemaFromClass(
      Class<?> clazz, Function<Class, List<TypeInformation>> getTypesForClass) {
    Schema.Builder builder = Schema.builder();
    for (TypeInformation type : getTypesForClass.apply(clazz)) {
      // TODO: look for nullable annotation.
      builder.addField(type.getName(), fieldFromType(type.getType(), getTypesForClass));
    }
    return builder.build();
  }

  // Map a Java field type to a Beam Schema FieldType.
  private static Schema.FieldType fieldFromType(
      TypeDescriptor type, Function<Class, List<TypeInformation>> getTypesForClass) {
    FieldType primitiveType = PRIMITIVE_TYPES.get(type.getRawType());
    if (primitiveType != null) {
      return primitiveType;
    }

    if (type.isArray()) {
      // If the type is T[] where T is byte, this is a BYTES type.
      TypeDescriptor component = type.getComponentType();
      if (component.getRawType().equals(byte.class)) {
        return FieldType.BYTES;
      } else {
        // Otherwise this is an array type.
        return FieldType.array(fieldFromType(component, getTypesForClass));
      }
    } else if (type.isSubtypeOf(TypeDescriptor.of(Collection.class))) {
      TypeDescriptor<Collection<?>> collection = type.getSupertype(Collection.class);
      if (collection.getType() instanceof ParameterizedType) {
        ParameterizedType ptype = (ParameterizedType) collection.getType();
        java.lang.reflect.Type[] params = ptype.getActualTypeArguments();
        checkArgument(params.length == 1);
        return FieldType.array(fieldFromType(TypeDescriptor.of(params[0]), getTypesForClass));
      } else {
        throw new RuntimeException("Cannot infer schema from unparameterized collection.");
      }
    } else if (type.isSubtypeOf(TypeDescriptor.of(Map.class))) {
      TypeDescriptor<Collection<?>> map = type.getSupertype(Map.class);
      if (map.getType() instanceof ParameterizedType) {
        ParameterizedType ptype = (ParameterizedType) map.getType();
        java.lang.reflect.Type[] params = ptype.getActualTypeArguments();
        checkArgument(params.length == 2);
        FieldType keyType = fieldFromType(TypeDescriptor.of(params[0]), getTypesForClass);
        FieldType valueType = fieldFromType(TypeDescriptor.of(params[1]), getTypesForClass);
        checkArgument(
            keyType.getTypeName().isPrimitiveType(), "Only primitive types can be map keys");
        return FieldType.map(keyType, valueType);
      } else {
        throw new RuntimeException("Cannot infer schema from unparameterized map.");
      }
    } else if (type.isSubtypeOf(TypeDescriptor.of(CharSequence.class))) {
      return FieldType.STRING;
    } else if (type.isSubtypeOf(TypeDescriptor.of(ReadableInstant.class))) {
      return FieldType.DATETIME;
    } else if (type.isSubtypeOf(TypeDescriptor.of(ByteBuffer.class))) {
      return FieldType.BYTES;
    } else {
      return FieldType.row(schemaFromClass(type.getRawType(), getTypesForClass));
    }
  }
}
