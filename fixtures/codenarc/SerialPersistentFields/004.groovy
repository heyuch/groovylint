
            // Wrong field type, JVM sees it as Object!
            class MyClass implements Serializable {
                static final public ObjectStreamField[] serialPersistentFields = [ new ObjectStreamField("myField", List.class) ] as ObjectStreamField[]
            }
        