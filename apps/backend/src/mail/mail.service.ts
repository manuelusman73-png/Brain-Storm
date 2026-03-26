import { Injectable, Logger } from '@nestjs/common';
import * as nodemailer from 'nodemailer';

@Injectable()
export class MailService {
  private readonly logger = new Logger(MailService.name);
  private transporter: nodemailer.Transporter;

  constructor() {
    this.transporter = nodemailer.createTransport({
      host: process.env.EMAIL_HOST,
      port: parseInt(process.env.EMAIL_PORT || '587'),
      secure: process.env.EMAIL_SECURE === 'true',
      auth: {
        user: process.env.EMAIL_USER,
        pass: process.env.EMAIL_PASS,
      },
    });
  }

  async sendVerificationEmail(to: string, token: string) {
    const frontendUrl = process.env.FRONTEND_URL || 'http://localhost:3001';
    const verifyUrl = `${frontendUrl}/auth/verify?token=${token}`;

    if (process.env.EMAIL_ENABLED !== 'true') {
      this.logger.log(`[DEV] Verification link for ${to}: ${verifyUrl}`);
      return;
    }

    await this.transporter.sendMail({
      from: process.env.EMAIL_FROM || '"Brain Storm" <no-reply@brainstorm.app>',
      to,
      subject: 'Verify your email',
      html: `<p>Click the link below to verify your email. It expires in 24 hours.</p>
             <a href="${verifyUrl}">${verifyUrl}</a>`,
    });
  }
}
