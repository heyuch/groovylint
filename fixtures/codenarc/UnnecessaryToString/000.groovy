
            def name = nameNode.toString()
            def id = idNode.lastChild.toString()
            def code = "$id-1234".toString()     // GString

            '(' + parameters?.collect { it?.type?.toString() }?.join(', ') + ')'
        