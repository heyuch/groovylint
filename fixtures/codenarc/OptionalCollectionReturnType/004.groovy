
            class MyClass {
                Optional<Collection<Object>> getCollection() { }
                private Optional<List<Integer>> getList() { }
                Optional<ArrayList<String>> getArrayList() { }
                Optional<LinkedList<Double>> getLinkedList() { }
                protected Optional<Set<BigDecimal>> getSet() { }
                Optional<HashSet<Boolean>> getHashSet() { }
                Optional<LinkedHashSet<Boolean>> getLinkedHashSet() { }
                Optional<TreeSet<Boolean>> getTreeSet() { }
                Optional<SortedSet<Boolean>> getSortedSet() { }
                Optional<EnumSet<MyEnum>> getEnumSet() { }
                Optional<Map<Integer, String>> getMap() { }
                Optional<HashMap<String, String>> getHashMap() { }
                Optional<LinkedHashMap<String, String>> getLinkedHashMap() { }
                Optional<EnumMap<String, String>> getEnumMap() { }
                Optional<SortedMap<String, String>> getSortedMap() { }
                Optional<TreeMap<String, String>> getTreeMap() { }
            }
        