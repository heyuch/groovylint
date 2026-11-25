
            // Wrong field type, JVM sees it as Object!
            class MyClass implements Serializable {
                private static final serialPersistentFields = [ new ObjectStreamField("myField", List.class) ] as ObjectStreamField[]
            }
        