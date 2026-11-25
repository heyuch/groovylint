
            package org.grails.datastore.gorm

            trait GormEntity<D> {
                D save() {
                    pritnln "saved $this"
                }
                static <D> List<D> list() {
                    Collections.emptyList()
                }
            }

            class SomeEntity implements GormEntity<SomeEntity> {

                SomeEntity prepare() {
                    save()
                }

            }

            class SomeService {

                void someMethod() {
                    List<SomeEntity> entities = SomeEntity.list()
                    SomeEntity firstEntity = list.first()
                    firstEntity.save()
                }

            }
        