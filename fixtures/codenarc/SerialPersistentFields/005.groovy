
            // Wrong field type, JVM sees it as Object!
            class MyClass implements Serializable {
                private final ObjectStreamField[] serialPersistentFields = [ new ObjectStreamField("myField", List.class) ] as ObjectStreamField[]
            }
        