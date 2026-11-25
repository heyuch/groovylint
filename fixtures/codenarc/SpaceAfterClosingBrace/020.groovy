
            def doStuff() {
                def things = new ObjectMapper().readValue('{}', new TypeReference<List<Thing>>() {})
            }
        