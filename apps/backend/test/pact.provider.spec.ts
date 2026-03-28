import { Test, TestingModule } from '@nestjs/testing';
import { INestApplication } from '@nestjs/common';
import { Verifier } from '@pact-foundation/pact';
import * as path from 'path';
import { AppModule } from '../../app.module';

describe('Pact Provider Verification', () => {
  let app: INestApplication;

  beforeAll(async () => {
    const moduleFixture: TestingModule = await Test.createTestingModule({
      imports: [AppModule],
    }).compile();

    app = moduleFixture.createNestApplication();
    await app.init();
  });

  afterAll(async () => {
    await app.close();
  });

  it('should verify pacts', async () => {
    const verifier = new Verifier({
      provider: 'BrainStorm-Backend',
      providerBaseUrl: 'http://localhost:3000',
      pactFiles: [path.resolve(__dirname, '../../pacts')],
      stateHandlers: {
        'user is authenticated': async () => {
          // Setup authenticated state
        },
        'course exists': async () => {
          // Setup course state
        },
      },
    });

    await verifier.verifyProvider();
  });
});
