
            def failEvents = messages.parallelStream()
                .filter { item -> item.headers.get( headerName ) == 'failed' }
                .collect ( Collectors.toList() ) as List<Message>
        